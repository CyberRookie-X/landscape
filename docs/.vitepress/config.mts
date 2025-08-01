import { defineConfig } from "vitepress";

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "Landscape Router",
  description: "Configuring Linux as a Router",
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: "主页", link: "/" },
      { text: "文档", link: "/introduction" },
    ],

    sidebar: [
      {
        text: "介绍",
        items: [
          { text: "简介", link: "/introduction" },
          { text: "手工部署", link: "/manual" },
          { text: "注意事项", link: "/attention" },
          { text: "部分界面截图", link: "/screenshot" },
        ],
      },
      {
        text: "特性",
        items: [
          { text: "分流控制", link: "/flow" },
          { text: "eBPF 路由", link: "/feature/route.md" },
        ],
      },
      {
        text: "配置",
        items: [{ text: "配置介绍", link: "/config/index.md" }],
      },
      {
        text: "编译",
        items: [
          { text: "编译", link: "/compilation/index.md" },
          { text: "与 Armbian 集成", link: "/compilation/armbian.md" },
          { text: "交叉编译", link: "/compilation/cross.md" },
        ],
      },
      {
        text: "FAQ",
        items: [
          { text: "DNS 服务相关", link: "/faq/dns.md" },
          // { text: "在已有网络基础上运行", link: "/faq/coexist.md" },
          { text: "与 iptable 的关系", link: "/faq/iptables.md" },
        ],
      },
      // {
      //   text: "拓扑",
      //   items: [
      //     { text: "网卡区域", link: "/iface/zone.md" },
      //     { text: "服务配置", link: "/iface/service.md" },
      //   ],
      // },
      // {
      //   text: "网卡服务",
      //   items: [
      //     { text: "静态 IP 配置 (WAN / LAN)", link: "/todo" },
      //     { text: "PPPD (WAN)", link: "/todo" },
      //     { text: "DHCP Client (WAN)", link: "/todo" },
      //     { text: "DHCP Server (LAN)", link: "/todo" },
      //   ],
      // },
      // {
      //   text: "系统服务",
      //   items: [
      //     { text: "DNS", link: "/todo" },
      //     { text: "IP 行为配置", link: "/todo" },
      //     { text: "Docker", link: "/todo" },
      //   ],
      // },
    ],

    socialLinks: [
      { icon: "github", link: "https://github.com/ThisSeanZhang/landscape" },
    ],
    footer: {
      message: "",
      copyright: "Copyright © 2025-present Sean",
    },
  },
});
