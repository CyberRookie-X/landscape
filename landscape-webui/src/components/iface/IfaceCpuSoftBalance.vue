<script setup lang="ts">
import { get_iface_cpu_balance, set_iface_cpu_balance } from "@/api/iface";
import { IfaceCpuSoftBalance } from "@/rust_bindings/common/iface";
import { ref } from "vue";
import CpuCoreSelector from "./CpuCoreSelector.vue";

const show_model = defineModel<boolean>("show", { required: true });
const loading = ref(false);
const props = defineProps<{
  iface_name: string;
}>();

const balance_config = ref<IfaceCpuSoftBalance>({
  xps: "",
  rps: "",
});

async function get_current_config() {
  let data = await get_iface_cpu_balance(props.iface_name);
  if (data) {
    balance_config.value = data;
  }
}

async function save_config() {
  try {
    loading.value = true;
    show_model.value = false;
    if (balance_config.value.xps !== "" || balance_config.value.rps !== "") {
      await set_iface_cpu_balance(props.iface_name, balance_config.value);
    }
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <n-modal
    :auto-focus="false"
    v-model:show="show_model"
    @after-enter="get_current_config"
  >
    <n-card
      style="width: 600px"
      title="配置网卡软负载"
      :bordered="false"
      size="small"
      role="dialog"
      aria-modal="true"
    >
      <n-flex vertical>
        <n-alert type="info">
          通过点击CPU核心编号来选择对应的CPU核心进行负载均衡配置。例如：
          要将负载分配给0号核心，点击"0"；要将负载分配给0、1、2号核心，分别点击"0"、"1"、"2"。
        </n-alert>
        <n-form v-if="balance_config" :model="balance_config">
          <n-form-item label="发送核心负载">
            <CpuCoreSelector v-model="balance_config.xps" />
          </n-form-item>
          <n-form-item label="接收核心负载">
            <CpuCoreSelector v-model="balance_config.rps" />
          </n-form-item>
        </n-form>
      </n-flex>

      <template #footer>
        <n-flex v-if="balance_config" justify="end">
          <n-button
            :loading="loading"
            round
            type="primary"
            @click="save_config"
          >
            更新
          </n-button>
        </n-flex>
      </template>
    </n-card>
  </n-modal>
</template>
