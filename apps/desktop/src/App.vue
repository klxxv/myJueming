<script setup lang="ts">
import brandIcon from "../../../assets/brand/jueming-aligner-icon-source.png";
</script>

<template>
  <main class="scaffold-shell">
    <img class="brand-icon" :src="brandIcon" alt="" />
    <div>
      <h1>决明对齐器</h1>
      <p>Jueming Aligner · 本地平行文本工作台</p>
      <span>Phase 1 workspace scaffold</span>
    </div>
  </main>
</template>

<style>
:root {
  font-family:
    "Segoe UI Variable", "Microsoft YaHei UI", "Segoe UI", sans-serif;
  color: #202521;
  background: #f7faf6;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
}

* {
  box-sizing: border-box;
}

html,
body,
#app {
  width: 100%;
  min-width: 1180px;
  height: 100%;
  min-height: 760px;
  margin: 0;
}

body {
  overflow: hidden;
}

.scaffold-shell {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  gap: 24px;
  color: #226f36;
  background:
    radial-gradient(circle at 50% 42%, #ffffff 0, #f7faf6 58%, #eef5eb 100%);
}

.brand-icon {
  width: 88px;
  height: 88px;
  border-radius: 22px;
  box-shadow: 0 10px 30px rgb(34 111 54 / 12%);
}

h1 {
  margin: 0 0 6px;
  font-size: 30px;
  letter-spacing: 0.02em;
}

p {
  margin: 0 0 10px;
  color: #4e5b50;
  font-size: 16px;
}

span {
  color: #7b877d;
  font-size: 13px;
}
</style>
