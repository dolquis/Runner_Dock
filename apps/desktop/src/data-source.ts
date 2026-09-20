/**
 * UI が読むデータの出所。
 *
 * Mock を既定にし、実 GitHub / 実 Agent への接続は明示的な opt-in にする。
 * Mock から本番接続へ自動的に落ちる経路を作らない
 * （docs/15_DEVELOPER_GUIDE.md §3）。
 */
export type DataSource = "mock" | "live";

/** 出所を選ぶ環境変数名。エラーメッセージ用の表示名であり、値の読取には使わない。 */
export const DATA_SOURCE_ENV = "VITE_RUNNERDOCK_DATA_SOURCE";

/** 出所の指定が解釈できないときのエラー。 */
export class UnknownDataSourceError extends Error {
  readonly received: string;

  constructor(received: string) {
    super(
      `${DATA_SOURCE_ENV}=${JSON.stringify(received)} を解釈できません。` +
        ` "mock" または "live" を指定してください。`,
    );
    this.name = "UnknownDataSourceError";
    this.received = received;
  }
}

/**
 * 環境変数の値からデータの出所を決める。
 *
 * 未指定と空文字は mock にする。未知の値は mock にも live にも丸めず失敗させる。
 * 誤記のまま実環境へ繋がる事故と、実環境のつもりで mock を見る事故の両方を防ぐ。
 */
export function resolveDataSource(raw: string | undefined): DataSource {
  const value = (raw ?? "").trim();
  if (value === "" || value === "mock") {
    return "mock";
  }
  if (value === "live") {
    return "live";
  }
  throw new UnknownDataSourceError(value);
}

/**
 * この build が読む出所。
 *
 * `import.meta.env` は静的メンバーアクセスで読む。Vite は動的添字を build 時に
 * 展開しないため、`import.meta.env[DATA_SOURCE_ENV]` と書くと production build
 * で値が落ち、dev サーバーとだけ挙動が一致する状態になる。
 */
export function currentDataSource(): DataSource {
  return resolveDataSource(import.meta.env.VITE_RUNNERDOCK_DATA_SOURCE);
}
