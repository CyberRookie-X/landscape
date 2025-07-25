// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.

export type NatConfig = {
  tcp_range: { start: number; end: number };
  udp_range: { start: number; end: number };
  icmp_in_range: { start: number; end: number };
};

export type NatServiceConfig = {
  iface_name: string;
  enable: boolean;
  nat_config: NatConfig;
  update_at: number;
};
