/// <reference types="vite/client" />

/**
 * この UI が読む環境変数。
 *
 * Vite は `import.meta.env` の**静的**メンバーアクセスだけを build 時に展開する。
 * 動的添字で読むと production build では値が落ち、dev と挙動が変わる。ここで
 * 型を宣言しておくと、静的アクセスのまま型検査を受けられる。
 */
interface ImportMetaEnv {
  readonly VITE_RUNNERDOCK_DATA_SOURCE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
