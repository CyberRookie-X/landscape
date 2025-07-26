<script setup lang="ts">
import { useSysInfo } from "@/stores/systeminfo";
import { computed, ref, watch } from "vue";

const props = defineProps<{
  modelValue?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const sysinfo = useSysInfo();

// 计算系统中的CPU核心数量
const cpuCount = computed(() => sysinfo.router_status.cpus.length);

// 创建核心选择状态数组
const coreSelections = ref<boolean[]>([]);

// 初始化核心选择状态
function initializeCoreSelections() {
  const coreCount = cpuCount.value;
  coreSelections.value = Array(coreCount).fill(false);
  
  // 如果有传入的值，则解析并设置选择状态
  if (props.modelValue) {
    const value = parseInt(props.modelValue, 10);
    if (!isNaN(value)) {
      // 将十进制数值转换为二进制表示，并更新选择状态
      for (let i = 0; i < coreCount; i++) {
        if (value & (1 << i)) {
          coreSelections.value[i] = true;
        }
      }
    }
  }
}

// 监听modelValue变化
watch(() => props.modelValue, () => {
  initializeCoreSelections();
}, { immediate: true });

// 监听CPU核心数量变化
watch(cpuCount, () => {
  initializeCoreSelections();
});

// 切换核心选择状态
function toggleCore(index: number) {
  coreSelections.value[index] = !coreSelections.value[index];
  updateModelValue();
}

// 更新绑定值
function updateModelValue() {
  let decimalValue = 0;
  for (let i = 0; i < coreSelections.value.length; i++) {
    if (coreSelections.value[i]) {
      decimalValue |= (1 << i);
    }
  }
  emit("update:modelValue", decimalValue.toString());
}

// 全选所有核心
function selectAll() {
  coreSelections.value = coreSelections.value.map(() => true);
  updateModelValue();
}

// 清除所有选择
function clearAll() {
  coreSelections.value = coreSelections.value.map(() => false);
  updateModelValue();
}
</script>

<template>
  <div>
    <div style="margin-bottom: 12px">
      <n-space>
        <n-button text type="primary" @click="selectAll">全选</n-button>
        <n-button text type="primary" @click="clearAll">清空</n-button>
      </n-space>
    </div>
    
    <n-grid :cols="8" :x-gap="8" :y-gap="8">
      <n-gi v-for="i in cpuCount" :key="i">
        <n-button 
          circle 
          :type="coreSelections[i-1] ? 'primary' : 'default'"
          @click="toggleCore(i-1)"
        >
          {{ i - 1 }}
        </n-button>
      </n-gi>
    </n-grid>
  </div>
</template>

<style scoped>
.cpu-core-selector {
  padding: 8px;
  border-radius: 4px;
  background-color: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.selector-controls {
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.core-grid {
  margin: 12px 0;
  padding: 8px 0;
}

.core-button {
  width: 32px;
  height: 32px;
  min-width: 32px;
  padding: 0;
  font-weight: bold;
}
</style>