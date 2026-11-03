<template>
  <svg class="brand-mark" viewBox="0 0 48 48" role="img" aria-hidden="true">
    <defs>
      <linearGradient id="bw-brand-gradient" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#3b9cff" />
        <stop offset="55%" stop-color="#1677ff" />
        <stop offset="100%" stop-color="#0958d9" />
      </linearGradient>
    </defs>
    <rect width="48" height="48" rx="13" fill="url(#bw-brand-gradient)" />
    <circle cx="34" cy="15" r="5" fill="#fff" fill-opacity="0.92" />
    <path
      d="M5 35.5 17.5 20l9 10.5 5.8-6.2L43 36v6H5z"
      fill="#fff"
      fill-opacity="0.94"
    />
    <path d="M5 41h38v2.4H5z" fill="#fff" fill-opacity="0.55" />
  </svg>
</template>

<style scoped>
.brand-mark {
  display: block;
  width: 34px;
  height: 34px;
  flex: none;
  filter: drop-shadow(0 6px 12px rgba(22, 119, 255, 0.28));
}
</style>
