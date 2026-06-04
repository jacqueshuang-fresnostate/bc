/// <reference types="vite/client" />

declare module '@tauri-apps/plugin-store' {
  interface StoreHandle {
    get(key: string): Promise<unknown>
    set(key: string, value: unknown): Promise<void>
    save(): Promise<void>
    clear(): Promise<void>
  }

  export function load(path: string): Promise<StoreHandle>
}
