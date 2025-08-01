use core::panic;
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    num::NonZeroUsize,
    path::PathBuf,
    sync::Arc,
    time::Instant,
    vec,
};

use hickory_proto::{
    op::{Header, ResponseCode},
    rr::{Record, RecordType},
};
use hickory_server::{
    authority::MessageResponseBuilder,
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
};
use lru::LruCache;
use tokio::sync::Mutex;

use crate::{diff_server::CheckDnsResult, rule::ResolutionRule, CacheDNSItem, DNSCache};
use landscape_common::{
    config::dns::{DNSRuntimeRule, FilterResult},
    flow::{DnsRuntimeMarkInfo, FlowDnsMarkInfo},
};

static RESOLVER_CONF: &'static str = "/etc/resolv.conf";
static RESOLVER_CONF_LD_BACK: &'static str = "/etc/resolv.conf.ld_back";

fn check_resolver_conf() {
    let resolver_file = PathBuf::from(RESOLVER_CONF);
    let resolver_file_back = PathBuf::from(RESOLVER_CONF_LD_BACK);
    let new_content = "nameserver 127.0.0.1\n";

    if resolver_file.is_symlink() {
        // 如果是符号链接，直接删除
        fs::remove_file(&resolver_file).unwrap();
    } else if resolver_file.exists() {
        if resolver_file.is_file() {
            // 如果是普通文件，检查备份文件
            if resolver_file_back.exists() {
                fs::remove_file(&resolver_file).unwrap();
            } else {
                let Ok(()) = fs::rename(&resolver_file, &resolver_file_back) else {
                    tracing::error!("move {resolver_file:?} error, Skip it");
                    return;
                };
            }
        } else {
            panic!("other kind file");
        }
    }

    // 写入新内容到 /etc/resolv.conf
    fs::write(&resolver_file, new_content).unwrap();
}

/// 整个 DNS 规则匹配树
#[derive(Clone, Debug)]
pub struct LandscapeDnsRequestHandle {
    /// 所有的域名处理对象
    /// 遍历的顺序是小到大
    resolves: BTreeMap<u32, Arc<ResolutionRule>>,
    pub cache: Arc<Mutex<DNSCache>>,
    pub flow_id: u32,
}

impl LandscapeDnsRequestHandle {
    pub fn new(dns_rules: Vec<DNSRuntimeRule>, flow_id: u32) -> LandscapeDnsRequestHandle {
        check_resolver_conf();
        let mut resolves = BTreeMap::new();
        for rule in dns_rules.into_iter() {
            // println!("dns_rules: {:?}", rule);
            resolves.insert(rule.index, Arc::new(ResolutionRule::new(rule, flow_id)));
        }
        let cache = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(2048).unwrap())));

        // landscape_ebpf::map_setting::flow::create_flow_dns_inner_map(flow_id, vec![]);
        LandscapeDnsRequestHandle { resolves, cache, flow_id }
    }

    pub fn renew_rules(&mut self, dns_rules: Vec<DNSRuntimeRule>) {
        check_resolver_conf();

        let mut resolves = BTreeMap::new();
        for rule in dns_rules.into_iter() {
            // println!("dns_rules: {:?}", rule);
            resolves.insert(rule.index, Arc::new(ResolutionRule::new(rule, self.flow_id)));
        }

        let mut cache = LruCache::new(NonZeroUsize::new(2048).unwrap());

        if let Ok(old_cache) = self.cache.try_lock() {
            let mut update_dns_mark_list: HashSet<FlowDnsMarkInfo> = HashSet::new();
            let mut del_dns_mark_list: HashSet<FlowDnsMarkInfo> = HashSet::new();

            for ((domain, req_type), value) in old_cache.iter() {
                'resolver: for (_index, resolver) in resolves.iter() {
                    if resolver.is_match(&domain) {
                        let new_mark = resolver.mark().clone();
                        // println!("old domain match resolver: {domain:?}");
                        let mut cache_items = vec![];
                        // println!(
                        //     "resolves: {:?}: match: {domain:?}, new_mark: {new_mark:?}",
                        //     resolver.config
                        // );
                        for cache_item in value.iter() {
                            // 新配置是 NoMark 的排除
                            match (
                                cache_item.mark.mark.need_insert_in_ebpf_map(),
                                new_mark.mark.need_insert_in_ebpf_map(),
                            ) {
                                (true, true) => {
                                    // 规则更新前后都需要写入 ebpf map
                                    // 因为现在是创建新的 map 所以即使一样也需要更新
                                    update_dns_mark_list
                                        .extend(cache_item.get_update_rules_with_mark(&new_mark));
                                }
                                (true, false) => {
                                    // 原先已经写入了
                                    // 现在不需要了
                                    del_dns_mark_list.extend(cache_item.get_update_rules());
                                }
                                (false, true) => {
                                    // 原先没写入
                                    // 目前需要缓存了
                                    update_dns_mark_list
                                        .extend(cache_item.get_update_rules_with_mark(&new_mark));
                                }
                                (false, false) => {
                                    // 原先没缓存，现在也不需要存储的
                                }
                            }

                            let mut new_record = cache_item.clone();
                            new_record.mark = new_mark.clone();
                            new_record.filter = resolver.filter_mode();
                            cache_items.push(new_record);
                        }
                        if !cache_items.is_empty() {
                            cache.push((domain.clone(), req_type.clone()), cache_items);
                        }
                        break 'resolver;
                    }
                }
            }
            tracing::info!("add_dns_marks: {:?}", update_dns_mark_list);

            landscape_ebpf::map_setting::flow_dns::create_flow_dns_inner_map(
                self.flow_id,
                update_dns_mark_list.into_iter().collect(),
            );
            // landscape_ebpf::map_setting::flow_dns::del_flow_dns_mark_rules(
            //     self.flow_id,
            //     del_dns_mark_list.into_iter().collect(),
            // );
        }

        // println!("cache: {:?}", cache);
        let cache = Arc::new(Mutex::new(cache));

        self.resolves = resolves;
        self.cache = cache;
    }

    pub async fn check_domain(&self, domain: &str, query_type: RecordType) -> CheckDnsResult {
        let mut result = CheckDnsResult::default();

        for (_index, resolver) in self.resolves.iter() {
            if resolver.is_match(&domain) {
                result.config = Some(resolver.get_runtime_config());
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(5),
                    resolver.lookup(&domain, query_type),
                )
                .await
                {
                    Ok(Ok(rdata_vec)) => {
                        result.records = Some(rdata_vec);
                    }
                    Ok(Err(_)) => {
                        // lookup 返回了错误
                    }
                    Err(_) => {
                        tracing::error!("check domain timeout")
                    }
                }
                break;
            }
        }

        if let Some((records, _)) = self.lookup_cache(domain, query_type).await {
            result.cache_records = Some(records);
        }

        result
    }

    // 检查缓存并根据 TTL 判断是否过期
    // 不同的记录可能的过期时间不同
    pub async fn lookup_cache(
        &self,
        domain: &str,
        query_type: RecordType,
    ) -> Option<(Vec<Record>, FilterResult)> {
        let mut cache = self.cache.lock().await;
        if let Some(records) = cache.get(&(domain.to_string(), query_type)) {
            let mut is_expire = false;
            let mut valid_records: Vec<Record> = vec![];
            let mut ret_fiter = FilterResult::Unfilter;
            'a_record: for CacheDNSItem { rdatas, insert_time, filter, .. } in records.iter() {
                ret_fiter = filter.clone();
                for rdata in rdatas.iter() {
                    if insert_time.elapsed().as_secs() > rdata.ttl() as u64 {
                        is_expire = true;
                        break 'a_record;
                    }

                    // tracing::debug!(
                    //     "query {domain} , filter: {filter:?}, result: {valid_records:?}"
                    // );
                }
                valid_records.extend_from_slice(rdatas);
            }

            if is_expire {
                return None;
            }

            // 如果有有效的记录，返回它们
            if !valid_records.is_empty() {
                return Some((valid_records, ret_fiter));
            }
        }
        None
    }

    pub async fn insert(
        &self,
        domain: &str,
        query_type: RecordType,
        rdata_ttl_vec: Vec<Record>,
        mark: &DnsRuntimeMarkInfo,
        filter: FilterResult,
    ) {
        let cache_item = CacheDNSItem {
            rdatas: rdata_ttl_vec,
            insert_time: Instant::now(),
            mark: mark.clone(),
            filter,
        };
        let update_dns_mark_list = cache_item.get_update_rules();

        let mut cache = self.cache.lock().await;
        cache.put((domain.to_string(), query_type), vec![cache_item]);
        drop(cache);
        // 将 mark 写入 mark ebpf map
        if mark.mark.need_insert_in_ebpf_map() {
            tracing::info!("setting ips: {:?}, Mark: {:?}", update_dns_mark_list, mark);
            // TODO: 如果写入错误 返回错误后 向客户端返回查询错误
            landscape_ebpf::map_setting::flow_dns::update_flow_dns_rule(
                self.flow_id,
                update_dns_mark_list.into_iter().collect(),
            );
        }
    }
}

#[async_trait::async_trait]
impl RequestHandler for LandscapeDnsRequestHandle {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        let queries = request.queries();
        if queries.is_empty() {
            let mut header = Header::response_from_request(request.header());
            header.set_response_code(ResponseCode::FormErr);
            let response =
                MessageResponseBuilder::from_message_request(request).build_no_records(header);
            let result = response_handle.send_response(response).await;
            return match result {
                Err(e) => {
                    tracing::error!("Request failed: {}", e);
                    serve_failed()
                }
                Ok(info) => info,
            };
        }

        // 先只处理第一个查询
        let req = &queries[0];
        let domain = req.name().to_string();
        let query_type = req.query_type();

        let response_builder = MessageResponseBuilder::from_message_request(request);
        let mut header = Header::response_from_request(request.header());
        header.set_response_code(ResponseCode::NoError);
        header.set_authoritative(true);
        header.set_recursion_available(true);

        let mut records = vec![];

        // TODO: 修改逻辑
        if let Some((result, filter)) = self.lookup_cache(&domain, query_type).await {
            records = fiter_result(result, &filter);
        } else {
            for (_index, resolver) in self.resolves.iter() {
                if resolver.is_match(&domain) {
                    records = match resolver.lookup(&domain, query_type).await {
                        Ok(rdata_vec) => {
                            if rdata_vec.len() > 0 {
                                self.insert(
                                    &domain,
                                    query_type,
                                    rdata_vec.clone(),
                                    resolver.mark(),
                                    resolver.filter_mode(),
                                )
                                .await;
                            }
                            fiter_result(rdata_vec, &resolver.filter_mode())
                        }
                        Err(error_code) => {
                            // 构建并返回错误响应
                            header.set_response_code(error_code);
                            let response = MessageResponseBuilder::from_message_request(request)
                                .build_no_records(header);
                            let result = response_handle.send_response(response).await;
                            return match result {
                                Err(e) => {
                                    tracing::error!("Request failed: {}", e);
                                    serve_failed()
                                }
                                Ok(info) => info,
                            };
                        }
                    };
                    break;
                }
            }
        }

        // 如果没有找到记录，返回 NXDomain 响应
        if records.is_empty() {
            // header.set_response_code(ResponseCode::NXDomain);
            let response = response_builder.build_no_records(header);
            let result = response_handle.send_response(response).await;
            return match result {
                Err(e) => {
                    tracing::error!("Record Empty and response error: {}", e);
                    serve_failed()
                }
                Ok(info) => info,
            };
        }

        let response = response_builder.build(
            header,
            records.iter(),
            vec![].into_iter(),
            vec![].into_iter(),
            vec![].into_iter(),
        );

        let result = response_handle.send_response(response).await;
        match result {
            Err(e) => {
                tracing::error!("Request failed: {}", e);
                serve_failed()
            }
            Ok(info) => info,
        }
    }
}

fn serve_failed() -> ResponseInfo {
    let mut header = Header::new();
    header.set_response_code(ResponseCode::ServFail);
    header.into()
}

fn fiter_result(un_filter_records: Vec<Record>, filter: &FilterResult) -> Vec<Record> {
    let mut valid_records = Vec::with_capacity(un_filter_records.len());
    for rdata in un_filter_records.into_iter() {
        if matches!(filter, FilterResult::Unfilter) {
            valid_records.push(rdata.clone());
        } else {
            match (rdata.record_type(), filter) {
                (RecordType::A, FilterResult::OnlyIPv4) => {
                    valid_records.push(rdata.clone());
                }
                (RecordType::A, FilterResult::OnlyIPv6)
                | (RecordType::AAAA, FilterResult::OnlyIPv4) => {
                    // 过滤
                }
                (RecordType::AAAA, FilterResult::OnlyIPv6) => {
                    valid_records.push(rdata.clone());
                }
                _ => {
                    valid_records.push(rdata.clone());
                }
            }
        }
    }
    valid_records
}
