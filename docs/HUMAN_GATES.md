# Human Gate 登録簿

AI セッションが単独で確定しない判断を登録する。各項目は、何を決めるか、なぜ人間が要るか、AI が引き渡す材料、通過の記録先を持つ。

AI は論点・選択肢・リスク・推奨案まで整理して人間へ渡す。CI やツール診断の成功を通過の証拠に置き換えない。通過の状態は Linear が正典で、この文書は定義だけを持つ（`AGENTS.md` §1）。Issue には `gate:human-required` を付ける。

## HG-01 GitHub App 作成と認証資格情報の発行

GitHub App の作成、Device Flow の client 登録、PAT の発行と scope 決定、失効操作。人間のアカウント権限と課金主体に紐づき、発行した値は secret であって repo に入らない。

- AI が渡すもの: `docs/06_GITHUB_AUTH_API.md`「必要権限」「使用 endpoint」から導いた最小 scope 案と、各 scope が要る理由。
- 記録先: Linear Issue のコメント。値そのものは書かない。

## HG-02 Runner 登録とサービス操作

self-hosted Runner の登録・削除、Windows サービスの登録と停止、WSL ディストリビューションの導入と削除。実行すると開発者の PC の既存環境を変える。

- AI が渡すもの: `docs/07_WINDOWS_WSL_BACKENDS.md`「既存環境を壊さない原則」に照らした影響範囲と、失敗時の復旧手順。
- 記録先: Linear Issue のコメント。

## HG-03 Windows / WSL 実機検証

起動維持、GUI 終了と Node 停止の分離、スリープ・再起動からの復帰、資源制約、障害注入。`docs/12_TEST_PLAN.md`「検証階層」の実機層に対応する。モックの成功で代替しない。

- AI が渡すもの: 実行するコマンド、期待する観測値、証跡の取り方（`docs/12_TEST_PLAN.md`「証跡フォーマット」）。
- 記録先: PR 本文の Validation と Linear の検証メモ。

## HG-04 branch protection と required check

`main` の branch protection、required status check の構成変更。AI は設定せず、必要な check 名と理由だけを申し送る。

- AI が渡すもの: 必須にしたい workflow / job 名と、それが守る不変条件。
- 記録先: Linear Issue のコメント。

## HG-05 コード署名と配布

署名証明書の取得と保管、署名値の設定、インストーラーの配布チャネル決定、公開リリース。`docs/16_OPERATIONS_RELEASE.md`「署名・配布」に対応する。秘密鍵を repo にも CI ログにも置かない。

- AI が渡すもの: 配布方式の選択肢と、それぞれの前提・制約・取り消し可能性。
- 記録先: `docs/17_ADR.md` の ADR と Linear Issue。

## HG-06 製品名と商標

製品名、コマンド名、配布名の最終決定と、商標・既存名との重複確認。`README.md` は製品配布開始前に行うと定める。

- AI が渡すもの: 候補名と、調べた範囲での衝突の有無および調査の限界。
- 記録先: `docs/17_ADR.md` の ADR。

## HG-07 参考実装の扱い

HomeRun その他の既存実装から、どこまでを参照とし、どこからを移植とみなすか。ライセンス上の判断を含む。`docs/18_REFERENCE_REVIEW.md` と ADR-001 に対応する。

- AI が渡すもの: 参照した範囲、得た知見、実装が独立していることの説明。
- 記録先: `docs/17_ADR.md` と PR 本文。
