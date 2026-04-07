import { defineStore } from 'pinia';
import { listen } from '@tauri-apps/api/event';

export interface FlashItem {
  id: string;
  enabled: boolean;
  offset: string;
  filePath: string;
  fileSize?: string;
  description?: string;
}

export interface LogEntry {
  text: string;
  level: 'info' | 'warn' | 'error' | 'success';
  timestamp: string;
}

export interface TargetInformation {
  chipType: string;
  revision: string;
  mac: string;
  crystal: string;
  features: string;
}

export type ChipProfile = 'esp32' | 'esp8266' | 'custom';

const SETTINGS_KEY = 'esp_cube_settings';

const DEFAULT_ESP32_FILES: FlashItem[] = [
  { id: 'bootloader', enabled: true, offset: '0x1000', filePath: '', description: 'Bootloader' },
  { id: 'partition', enabled: true, offset: '0x8000', filePath: '', description: 'Partition Table' },
  { id: 'boot_app0', enabled: true, offset: '0xe000', filePath: '', description: 'Boot App (OTA data)' },
  { id: 'firmware', enabled: true, offset: '0x10000', filePath: '', description: 'Main Application' },
];

let isInitialized = false;
let timerInterval: any = null;

export const useMainStore = defineStore('main', {
  state: () => ({
    port: '',
    baudrate: '460800',
    ports: [] as string[],
    isScanning: false,
    chipInfo: null as TargetInformation | null,

    chipProfile: 'esp32' as ChipProfile,
    flashMode: 'dio',
    flashFreq: '40m',
    flashSize: 'keep',
    eraseBefore: false,
    verifyAfter: true,
    extraArgs: '',
    flashItems: JSON.parse(JSON.stringify(DEFAULT_ESP32_FILES)) as FlashItem[],

    status: 'idle',
    progress: 0,
    isBusy: false,
    elapsedTime: '00:00:00',
    startTime: 0,
    logs: [] as LogEntry[],
    activeTab: 'memory',
  }),
  actions: {
    addLog(text: string, level: LogEntry['level'] = 'info') {
      this.logs.push({ text, level, timestamp: new Date().toLocaleTimeString() });
      if (this.logs.length > 1000) this.logs.shift();
    },
    clearLogs() {
      this.logs = [];
    },
    startTimer() {
      this.stopTimer();
      this.startTime = Date.now();
      this.elapsedTime = '00:00:00';
      timerInterval = setInterval(() => {
        const diff = Date.now() - this.startTime;
        const h = Math.floor(diff / 3600000).toString().padStart(2, '0');
        const m = Math.floor((diff % 3600000) / 60000).toString().padStart(2, '0');
        const s = Math.floor((diff % 60000) / 1000).toString().padStart(2, '0');
        this.elapsedTime = `${h}:${m}:${s}`;
      }, 1000);
    },
    stopTimer() {
      if (timerInterval) {
        clearInterval(timerInterval);
        timerInterval = null;
      }
    },
    setProfile(profile: ChipProfile) {
      this.chipProfile = profile;
      if (profile === 'esp32') {
        this.flashItems = JSON.parse(JSON.stringify(DEFAULT_ESP32_FILES));
      } else if (profile === 'esp8266') {
        this.flashItems = [{ id: 'firmware', enabled: true, offset: '0x0', filePath: '', description: 'Main Firmware' }];
      } else {
        this.flashItems = [{ id: 'custom-1', enabled: true, offset: '0x0', filePath: '', description: 'Custom 1' }];
      }
    },
    addFlashItem() {
      this.flashItems.push({ id: Math.random().toString(36).substring(2, 9), enabled: true, offset: '0x0', filePath: '', description: `Custom ${this.flashItems.length + 1}` });
    },
    removeFlashItem(id: string) {
      if (this.flashItems.length > 1) this.flashItems = this.flashItems.filter(i => i.id !== id);
    },
    resetDefaultFiles() { this.setProfile(this.chipProfile); },
    saveSettings() {
      try {
        const s = { port: this.port, baudrate: this.baudrate, chipProfile: this.chipProfile, flashMode: this.flashMode, flashFreq: this.flashFreq, flashSize: this.flashSize, eraseBefore: this.eraseBefore, verifyAfter: this.verifyAfter, extraArgs: this.extraArgs, flashItems: this.flashItems };
        localStorage.setItem(SETTINGS_KEY, JSON.stringify(s));
      } catch (e) { console.error(e); }
    },
    loadSettings() {
      try {
        const saved = localStorage.getItem(SETTINGS_KEY);
        if (saved) {
          const p = JSON.parse(saved);
          this.port = p.port || ''; this.baudrate = p.baudrate || '460800'; this.chipProfile = p.chipProfile || 'esp32'; this.flashMode = p.flashMode || 'dio'; this.flashFreq = p.flashFreq || '40m'; this.flashSize = p.flashSize || 'keep'; this.eraseBefore = !!p.eraseBefore; this.verifyAfter = p.verifyAfter !== undefined ? p.verifyAfter : true; this.extraArgs = p.extraArgs || '';
          if (p.flashItems) this.flashItems = p.flashItems;
        }
      } catch (e) { console.error(e); }
    },
    async initListeners() {
      if (isInitialized) return;
      isInitialized = true;

      await listen('log_line', (event: any) => this.addLog(event.payload.text, event.payload.level));
      await listen('progress', (event: any) => { this.progress = event.payload.percent; });
      await listen('status_change', (event: any) => { 
        this.status = event.payload.status;
        // Logic mới: Kích hoạt timer ngay khi status không phải các trạng thái dừng
        const stopStates = ['idle', 'done', 'error'];
        if (!stopStates.includes(this.status)) {
           if (!timerInterval) {
              this.isBusy = true;
              this.startTimer();
           }
        }
      });
      await listen('completed', (event: any) => {
        this.isBusy = false;
        this.stopTimer();
        if (event.payload.success) {
          this.status = 'done';
          this.addLog('Operation finished successfully!', 'success');
        } else {
          this.status = 'error';
          this.addLog('Operation failed!', 'error');
        }
      });
    }
  }
});
