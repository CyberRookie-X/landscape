#include "landscape.h"
#include "share_ifindex_ip.h"
#include "firewall_share.h"
#include "flow_lan_share.h"
#include "flow_verdict_share.h"
#include "flow.h"
#include "metric.h"

char LICENSE[] SEC("license") = "Dual BSD/GPL";

SEC("tc/ingress")
int placeholder(struct __sk_buff *skb) { return TC_ACT_UNSPEC; }
