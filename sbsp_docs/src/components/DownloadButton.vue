<script setup>
import { ref, onMounted, computed } from 'vue'

const latestData = ref(null)
const version = ref('')
const userOS = ref('');
const isLoading = ref(true)
const isError = ref(false)

const detectOS = () => {
  const ua = window.navigator.userAgent.toLowerCase()
  if (ua.includes('win')) return 'windows'
  if (ua.includes('mac')) return 'mac'
  if (ua.includes('linux')) return 'linux'
  return ''
}

const linkData = computed(() => {
  if (latestData.value == null) {
    return {
      win: null,
      mac: null,
      linux_appimage: null,
      linux_deb: null,
      linux_rpm: null,
    };
  }

  const baseMacUrl = latestData.value.platforms["darwin-aarch64"]?.url ?? null;
  const dmgUrl = baseMacUrl != null ? baseMacUrl.replace(/[^/]+$/, `SBS.Player_${version.value}_universal.dmg`) : null;

  return {
    win: latestData.value.platforms['windows-x86_64']?.url ?? null,
    mac: dmgUrl,
    linux_appimage: latestData.value.platforms['linux-x86_64-appimage']?.url ?? null,
    linux_deb: latestData.value.platforms['linux-x86_64-deb']?.url ?? null,
    linux_rpm: latestData.value.platforms['linux-x86_64-rpm']?.url ?? null,
  }
})

onMounted(async () => {
  userOS.value = detectOS();
  try {
    const response = await fetch(`https://keinsleif.github.io/sbsp/update/latest-main.json`)
    if (!response.ok) throw new Error('Network response was not ok')
    
    const data = await response.json()
    
    if (data != null) {
      latestData.value = data;
      version.value = data.version
    } else {
      throw new Error('Asset not found')
    }
  } catch (err) {
    console.error('Failed to fetch latest release:', err)
    isError.value = true
  } finally {
    isLoading.value = false
  }
})
</script>

<template>
  <div :class="$style['download-container']">
    <div v-if="isLoading" :class="$style['loading']">最新バージョンを確認中...</div>
    
    <div v-else-if="isError" :class="$style['error']">
      情報を取得できませんでした。<br>
      <a :href="`https://github.com/Keinsleif/sbsp/releases/latest`" target="_blank" rel="noopener">
        GitHubのリリース一覧ページ
      </a> から直接ダウンロードしてください。
    </div>
    
    <div v-else>
      <div :class="$style['version-badge']">最新版: {{ version }}</div>
      <div :class="$style['download-card-container']">
        <div :class="[$style['download-card'], userOS === 'windows' ? $style['recommended'] : '']">
          <span style="display: flex; flex-direction: row; gap: 0.5em; justify-content: center; align-items: center;">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4875 4875" width="18px">
              <path fill="currentColor" d="M0 0h2311v2310H0zm2564 0h2311v2310H2564zM0 2564h2311v2311H0zm2564 0h2311v2311H2564"></path>
            </svg>
            Windows
          </span>
          <div style="display: flex; flex-direction: column; gap: 1em; padding: 1em;">
            <VPButton :href="linkData.win ?? '#'" :theme="userOS === 'windows' ? 'brand' : 'alt'">
              <span style="display: flex; flex-direction: row; gap: 0.5em; justify-content: center; align-items: center;">
                Windows
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="2 2 22 22" width="18"><title>download</title><path d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z" fill="currentColor" /></svg>
              </span>
            </VPButton>
          </div>
        </div>
        <div :class="[$style['download-card'], userOS === 'mac' ? $style['recommended'] : '']">
          <span style="display: flex; flex-direction: row; gap: 0.5em; justify-content: center; align-items: center;">
            <svg xmlns="http://www.w3.org/2000/svg" xml:space="preserve" viewBox="0 0 814 1000" width="18px">
              <path d="M788.1 340.9c-5.8 4.5-108.2 62.2-108.2 190.5 0 148.4 130.3 200.9 134.2 202.2-.6 3.2-20.7 71.9-68.7 141.9-42.8 61.6-87.5 123.1-155.5 123.1s-85.5-39.5-164-39.5c-76.5 0-103.7 40.8-165.9 40.8s-105.6-57-155.5-127C46.7 790.7 0 663 0 541.8c0-194.4 126.4-297.5 250.8-297.5 66.1 0 121.2 43.4 162.7 43.4 39.5 0 101.1-46 176.3-46 28.5 0 130.9 2.6 198.3 99.2zm-234-181.5c31.1-36.9 53.1-88.1 53.1-139.3 0-7.1-.6-14.3-1.9-20.1-50.6 1.9-110.8 33.7-147.1 75.8-28.5 32.4-55.1 83.6-55.1 135.5 0 7.8 1.3 15.6 1.9 18.1 3.2.6 8.4 1.3 13.6 1.3 45.4 0 102.5-30.4 135.5-71.3z" fill="currentColor"></path>
            </svg>
            macOS
          </span>
          <div style="display: flex; flex-direction: column; gap: 1em; padding: 1em;">
            <VPButton :href="linkData.mac ?? '#'" :theme="userOS === 'mac' ? 'brand' : 'alt'">
              <span style="display: flex; flex-direction: row; gap: 0.5em; justify-content: center; align-items: center;">
              macOS
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="2 2 22 22" width="18"><title>download</title><path d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z" fill="currentColor" /></svg>
              </span>
            </VPButton>
          </div>
        </div>
        <div :class="[$style['download-card'], userOS === 'linux' ? $style['recommended'] : '']">
          <span style="display: flex; flex-direction: row; gap: 0.5em; justify-content: center; align-items: center;">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 58 58" fill="none" width="18px">
              <path d="M47.6083 42.5333C47.3667 42.05 47.125 41.5667 47.125 41.0833C47.125 40.1167 46.6417 39.3917 45.9167 38.6667C45.675 38.425 45.1917 38.1833 44.95 38.1833C46.4 33.8333 44.225 29.4833 41.8083 26.3417C39.875 23.4417 36.975 21.2667 37.2167 17.4C37.2167 12.8083 37.7 4.35002 29.2417 5.07502C20.5417 5.55835 22.9583 14.5 22.7167 17.6417C22.7167 20.3 21.5083 22.9583 19.575 25.1333C19.0917 25.6167 18.6083 26.3417 18.3667 26.825C15.95 29.725 14.7417 33.5917 14.7417 37.2167C14.2583 37.7 13.775 38.1833 13.5333 38.6667C13.2917 38.9083 13.05 39.15 13.05 39.3917C12.8083 39.6333 12.325 39.875 11.8417 40.1167C10.875 40.3583 10.15 40.8417 9.66667 41.8083C9.425 42.5333 9.18333 43.5 9.425 44.4667C9.66667 44.95 9.66667 45.4333 9.425 46.1583C8.94167 47.125 8.94167 48.3333 9.425 49.5417C10.15 50.5083 11.3583 50.75 13.05 50.9917C14.2583 50.9917 15.7083 51.475 16.9167 51.9583C18.125 52.6833 19.575 53.1667 21.025 53.1667C21.75 53.1667 22.7167 52.925 23.4417 52.6833C24.1667 52.2 24.65 51.7167 24.8917 50.9917C25.8583 50.9917 27.3083 50.5083 29 50.5083C30.45 50.5083 31.9 50.9917 33.8333 50.75C33.8333 50.9917 33.8333 51.2333 34.075 51.475C34.5583 52.6833 35.7667 53.65 37.2167 53.8917H37.7C39.6333 53.65 41.5667 52.6833 42.775 51.2333C43.7417 50.2667 44.95 49.5417 46.1583 49.0583C47.6083 48.3333 48.575 47.85 48.8167 46.6417C49.0583 44.95 48.575 43.9833 47.6083 42.5333ZM30.9333 11.6C32.3833 11.8417 33.5917 13.05 33.35 14.5C33.35 15.225 33.1083 15.95 32.625 16.675H32.3833C31.9 16.4333 31.6583 16.4333 31.4167 16.1917C31.6583 15.95 31.6583 15.4667 31.9 14.9833C31.9 14.0167 31.4167 13.2917 30.9333 13.2917C30.2083 13.2917 29.725 14.0167 29.725 14.9833V15.225C29.4833 14.9833 29 14.9833 28.7583 14.7417V14.5C28.5167 13.2917 29.4833 11.8417 30.9333 11.6ZM30.2083 16.4333C30.45 16.675 30.9333 16.9167 31.175 16.9167C31.4167 16.9167 31.9 17.1583 32.1417 17.4C32.625 17.6417 33.1083 17.8833 33.1083 18.6083C33.1083 19.3333 32.3833 20.0583 30.9333 20.5417C30.45 20.7833 30.2083 20.7833 29.9667 21.025C29.2417 21.5083 28.5167 21.75 27.55 21.75C26.825 21.75 26.1 21.2667 25.6167 20.7833C25.375 20.5417 25.1333 20.3 24.65 20.0583C24.4083 19.8167 23.925 19.3333 23.6833 18.6083C23.6833 18.3667 23.925 18.125 24.1667 17.8833C24.8917 17.4 25.1333 17.1583 25.375 16.9167L25.6167 16.675C26.1 15.95 27.0667 15.4667 28.0333 15.4667C28.7583 15.7083 29.4833 15.95 30.2083 16.4333ZM25.1333 12.0833C26.1 12.0833 26.825 13.05 27.0667 14.7417V15.225C26.825 15.225 26.3417 15.4667 26.1 15.7083V15.225C26.1 14.5 25.6167 13.775 25.1333 14.0167C24.65 14.0167 24.4083 14.7417 24.4083 15.4667C24.4083 15.95 24.65 16.1917 24.8917 16.4333C24.8917 16.4333 24.65 16.675 24.4083 16.675C23.925 16.1917 23.4417 15.4667 23.4417 14.7417C23.4417 13.2917 24.1667 12.0833 25.1333 12.0833ZM22.7167 50.9917C21.025 51.7167 18.85 51.475 17.4 50.5083C15.95 49.7833 14.7417 49.5417 13.05 49.5417C11.8417 49.3 10.6333 49.3 10.3917 48.8167C10.15 48.3333 10.15 47.6083 10.6333 46.4C10.875 45.675 10.875 44.95 10.6333 44.225C10.3917 43.5 10.3917 43.0167 10.6333 42.2917C10.875 41.5667 11.3583 41.325 12.0833 41.0833C12.8083 40.8417 13.2917 40.6 13.775 40.1167C14.0167 39.875 14.2583 39.6334 14.5 39.15C15.225 38.1833 15.7083 37.7 16.4333 37.7C17.8833 37.9417 19.0917 40.1167 20.0583 42.2917C20.5417 43.0167 21.025 43.9833 21.75 44.7083C22.7167 45.9167 23.925 47.6083 23.925 48.575C23.925 49.7833 23.4417 50.5083 22.7167 50.9917ZM34.5583 45.675C34.5583 45.9167 34.5583 45.9167 34.3167 46.1583C31.4167 48.3333 27.55 48.575 24.4083 46.8833L22.9583 44.7083C25.1333 44.4667 24.65 41.5667 20.0583 38.6667C15.225 35.525 18.6083 29.725 20.3 27.0667C20.5417 26.825 20.5417 27.0667 19.575 29C18.85 30.45 17.4 34.075 19.3333 36.7333C19.3333 34.8 19.8167 32.8667 20.5417 30.9333C22.2333 27.7917 23.4417 24.1667 24.1667 20.5417C24.4083 20.7833 24.4083 20.7833 24.65 20.7833C24.8917 21.025 25.1333 21.2667 25.375 21.2667C25.8583 21.9917 26.825 22.2333 27.55 22.2333H27.7917C28.7583 22.2333 29.725 21.9917 30.45 21.2667C30.6917 21.025 30.9333 20.7833 31.4167 20.7833C32.1417 20.5417 32.8667 20.0583 33.5917 19.3333C34.5583 22.475 35.525 25.375 36.975 28.0333C37.9417 29.9667 38.6667 31.9 39.15 34.075C39.875 34.075 40.8417 34.3167 41.5667 34.8C43.5 35.7667 44.225 36.4917 43.9833 37.7H43.5C43.5 36.975 43.0167 36.25 41.325 35.525C39.6333 34.8 38.1833 34.8 37.7 36.4917C37.4583 36.4917 37.2167 36.7333 36.975 36.7333C35.0417 37.7 35.0417 40.3583 34.8 43.0167C35.0417 43.9833 34.8 44.7083 34.5583 45.675ZM45.675 47.125C44.225 47.6083 43.0167 48.575 42.05 49.7833C41.0833 51.2333 39.3917 52.2 37.4583 51.9583C36.4917 51.9583 35.525 51.2333 35.2833 50.2667C35.0417 48.8167 35.0417 47.3667 35.7667 45.9167C36.0083 44.95 36.25 44.225 36.4917 43.2583C36.7333 40.3583 36.7333 38.6667 37.9417 37.9417C37.9417 39.15 38.6667 39.875 39.6333 40.3583C40.8417 40.3583 42.05 40.1167 43.0167 39.15H43.5C44.225 39.15 44.7083 39.15 45.1917 39.6333C45.675 40.1167 45.9167 40.8417 45.9167 41.325C45.9167 42.05 46.4 42.775 46.6417 43.5C47.85 44.7083 47.85 45.4333 47.85 45.675C47.6083 46.1583 46.6417 46.6417 45.675 47.125ZM23.925 18.125C23.6833 18.125 23.6833 18.125 23.6833 18.3667C23.6833 18.3667 23.6833 18.6083 23.925 18.6083C24.1667 18.6083 24.1667 18.85 24.1667 18.85C24.8917 19.8167 26.1 20.3 27.55 20.5417C28.7583 20.3 29.9667 20.0583 31.175 19.0917L32.625 18.3667C32.8667 18.3667 32.8667 18.125 32.8667 18.125C32.8667 17.8833 32.8667 17.8833 32.625 17.8833C32.1417 18.125 31.4167 18.3667 30.9333 18.6083C29.9667 19.3333 28.7583 19.8167 27.55 19.8167C26.3417 19.8167 25.375 19.0917 24.65 18.3667C24.4083 18.3667 24.1667 18.125 23.925 18.125Z" fill="currentColor"></path>
            </svg>
            Linux
          </span>
          <div style="display: flex; flex-direction: column; gap: 1em; padding: 1em;">
            <VPButton :href="linkData.linux_deb ?? '#'" :theme="userOS === 'linux' ? 'brand' : 'alt'">
              <span style="display: flex; flex-direction: row; gap: 0.5em; justify-content: center; align-items: center;">
              Debian/Ubuntu
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="2 2 22 22" width="18"><title>download</title><path d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z" fill="currentColor" /></svg>
              </span>
            </VPButton>
            <VPButton :href="linkData.linux_rpm ?? '#'" :theme="userOS === 'linux' ? 'brand' : 'alt'">
              <span style="display: flex; flex-direction: row; gap: 0.5em; justify-content: center; align-items: center;">
              Red Hat/Fedora
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="2 2 22 22" width="18"><title>download</title><path d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z" fill="currentColor" /></svg>
              </span>
            </VPButton>
            <VPButton :href="linkData.linux_appimage ?? '#'" :theme="userOS === 'linux' ? 'brand' : 'alt'">
              <span style="display: flex; flex-direction: row; gap: 0.5em; justify-content: center; align-items: center;">
              AppImage
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="2 2 22 22" width="18"><title>download</title><path d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z" fill="currentColor" /></svg>
              </span>
            </VPButton>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style module>
.download-container {
  margin: 2rem 0;
}
.download-card-container {
  display: flex;
  align-items: stretch;
  flex-direction: row;
  gap: 1.5rem;
}
@media screen and (max-width: 800px) {
  .download-card-container {
    display: flex;
    align-items: stretch;
    flex-direction: column;
    gap: 1.5rem;
  }
}
.download-card {
  padding: 1.5rem;
  border: 1px solid var(--vp-c-bg-alt);
  background-color: var(--vp-c-bg-soft);
  border-radius: 12px;
  text-align: center;
  box-shadow: 0 4px 12px rgba(0,0,0,0.05);
}
.download-card.recommended {
  border: 2px solid var(--vp-c-brand-1);
  background-color: var(--vp-c-brand-soft);
}
.version-badge {
  text-align: left;
}
.file-name {
  font-size: 0.9rem;
  color: var(--vp-c-text-2);
  margin-bottom: 1rem;
  word-break: break-all;
}
.download-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  background-color: var(--vp-c-brand-1);
  color: white !important;
  padding: 0.75rem 2rem;
  border-radius: 8px;
  font-weight: 600;
  text-decoration: none !important;
  transition: background-color 0.2s, transform 0.1s;
}
.download-btn:hover {
  background-color: var(--vp-c-brand-2);
  transform: translateY(-1px);
}
.download-btn:active {
  transform: translateY(1px);
}
.loading, .error {
  text-align: center;
  padding: 2rem;
  color: var(--vp-c-text-2);
}
</style>