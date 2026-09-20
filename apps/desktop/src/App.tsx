import { useEffect, useState } from "react";

import { currentDataSource } from "./data-source";
import { type ShellPeer, fetchShellPeer } from "./shell-peer";

/**
 * LF-001 の最小画面。
 *
 * Runner の一覧、操作ボタン、GitHub 接続は持たない。workspace が起動し、
 * どの出所のデータを読む build なのかを確認するためだけの画面である。
 */
export function App(): React.JSX.Element {
  const dataSource = currentDataSource();
  const [shellPeer, setShellPeer] = useState<ShellPeer | null>(null);

  useEffect(() => {
    let cancelled = false;
    void fetchShellPeer().then((peer) => {
      if (!cancelled) {
        setShellPeer(peer);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="app">
      <h1>Runner Dock</h1>
      <p className="lead">
        開発用の最小画面です。Runner の操作、GitHub 認証、WSL の変更は行いません。
      </p>
      <dl className="facts">
        <dt>データの出所</dt>
        <dd data-testid="data-source">{dataSource}</dd>
        <dt>Desktop shell</dt>
        <dd data-testid="shell-peer">
          {shellPeer === null
            ? "未接続（ブラウザ起動）"
            : `protocol ${shellPeer.protocolMajor} / ${shellPeer.implementationVersion}`}
        </dd>
      </dl>
    </main>
  );
}
