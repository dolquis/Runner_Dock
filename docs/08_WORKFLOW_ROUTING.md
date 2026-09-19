# 08. Workflow Assistantと事前ルーティング

**区分:** 規範＋参考テンプレート / **MVP:** 生成・説明・保存。既存Workflow自動変更は次段階

## 1. 提供する機能の正確な名前

製品内では「ローカル優先・事前選択」と呼びます。オンライン・空き状態を調べてから、後続jobの`runs-on`を決めます。GitHubの実行開始後に別OS環境へジョブを移す機能ではありません。

GitHubではRunnerが見つからないjobはキューに残り得ます。配列の`runs-on`は代替候補の列挙ではなく、label条件の組合せです。60秒の再キューや24時間の上限を「hostedへ切替わる機能」と解釈しません。[S01](19_SOURCES.md#s01)[S22](19_SOURCES.md#s22)

## 2. 信頼判定を可用性判定より先に行う

```text
イベント・repo・ref・ポリシーが許可されているか
  ├─ NO → hostedのみ、または課金方針により拒否
  └─ YES → 対象Runner ID / labels / online / idle を確認
             ├─ 利用可能 → local labels
             └─ 不可/不明 → hosted許可ならhosted、許可なしなら明示失敗
```

MVPの既定対象は承認済みprivate repoの保護されたmainへのpushと、そのrefでの手動実行です。PRは同一repo内を含めhostedのみです。`pull_request_target`で未信頼のheadをcheckoutする生成はしません。

ただしWorkflowの条件は敵対的な利用者に対する強固な境界ではありません。別Workflowがself-hosted labelを直接指定できるなら迂回され得ます。登録対象repo・書込権限・Runner group方針を別途管理します。[S19](19_SOURCES.md#s19)

## 3. 費用方針

`allow_hosted=true`は利用者が適用前に明示同意した場合だけ生成します。selector自身もhosted上で実行されるため、ローカルビルドになった場合でもhosted利用が完全にゼロとは限りません。[S20](19_SOURCES.md#s20)

`local_only`では利用不能時に長時間待たせるか、失敗させるかを別のpolicyとして指定します。MVP推奨は明示失敗です。節約設定なのにAPI障害で有料Runnerへ黙って切替えることを禁止します。

## 4. テンプレートが成立する条件

対象Runnerはrepo-levelとして作成済みで、WindowsとLinuxそれぞれのremote Runner IDとunique custom labelが確定していること。GitHub Actions Secret `RUNNER_STATUS_TOKEN`に対象repo限定のAdministration read資格情報を入れること。buildの依存ツールは両環境に用意すること。[S02](19_SOURCES.md#s02)

以下は構成確認用のテンプレートです。`OWNER/REPO`、`12345`、`67890`、custom labelは**説明用の値**であり、実装時は検証済み設定から生成します。hosted許可を選んだ場合の例で、APIエラー時もその同意に基づいてhostedを選びます。

```yaml
name: LocalForge routing smoke test
on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:
    inputs:
      force_hosted:
        description: Use GitHub-hosted for this run
        type: boolean
        default: false

permissions:
  contents: read

jobs:
  choose:
    runs-on: ubuntu-24.04
    timeout-minutes: 3
    outputs:
      windows: ${{ steps.local.outputs.windows || '["windows-2022"]' }}
      linux: ${{ steps.local.outputs.linux || '["ubuntu-24.04"]' }}
    steps:
      - name: Inspect approved local runners
        id: local
        if: >-
          github.repository == 'OWNER/REPO' &&
          github.event.repository.private == true &&
          github.ref == 'refs/heads/main' &&
          github.ref_protected == true &&
          (github.event_name == 'push' || github.event_name == 'workflow_dispatch') &&
          !inputs.force_hosted
        shell: bash
        env:
          GH_TOKEN: ${{ secrets.RUNNER_STATUS_TOKEN }}
          TARGET_REPO: ${{ github.repository }}
          WINDOWS_RUNNER_ID: '12345'
          LINUX_RUNNER_ID: '67890'
        run: |
          python3 - <<'PYTHON'
          import json, os, urllib.request, urllib.error

          token = os.environ.get("GH_TOKEN", "")
          repo = os.environ["TARGET_REPO"]
          targets = [
              ("windows", os.environ["WINDOWS_RUNNER_ID"],
               ["self-hosted", "Windows", "X64", "localforge-home-windows"],
               ["windows-2022"]),
              ("linux", os.environ["LINUX_RUNNER_ID"],
               ["self-hosted", "Linux", "X64", "localforge-home-ubuntu"],
               ["ubuntu-24.04"]),
          ]

          # Credential-bearing requests must not follow redirects.
          class NoRedirect(urllib.request.HTTPRedirectHandler):
              def redirect_request(self, req, fp, code, msg, headers, newurl):
                  return None

          opener = urllib.request.build_opener(NoRedirect)
          for key, runner_id, local_labels, hosted_labels in targets:
              selected = hosted_labels
              reason = "missing-monitor-token" if not token else "invalid-runner-id"
              if token and runner_id.isdecimal():
                  request = urllib.request.Request(
                      f"https://api.github.com/repos/{repo}/actions/runners/{runner_id}",
                      headers={
                          "Accept": "application/vnd.github+json",
                          "Authorization": f"Bearer {token}",
                          "X-GitHub-Api-Version": "2026-03-10",
                          "User-Agent": "LocalForge-Workflow-Selector",
                      },
                  )
                  try:
                      with opener.open(request, timeout=10) as response:
                          item = json.load(response)
                      if not isinstance(item, dict) or not isinstance(item.get("labels"), list):
                          raise ValueError("invalid runner schema")
                      labels = set()
                      for label in item["labels"]:
                          if not isinstance(label, dict) or not isinstance(label.get("name"), str):
                              raise ValueError("invalid label schema")
                          labels.add(label["name"].lower())
                      ready = (
                          str(item.get("id")) == runner_id
                          and item.get("status") == "online"
                          and item.get("busy") is False
                          and all(x.lower() in labels for x in local_labels)
                      )
                      if ready:
                          selected, reason = local_labels, "observed-online-idle"
                      else:
                          reason = "unavailable-or-label-mismatch"
                  except urllib.error.HTTPError as error:
                      reason = f"api-http-{error.code}"
                  except (urllib.error.URLError, TimeoutError, ValueError,
                          KeyError, TypeError, OSError):
                      reason = "api-unavailable-or-invalid-response"
              with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
                  output.write(f"{key}={json.dumps(selected, separators=(',', ':'))}\n")
              print(f"{key}: {reason}")
          PYTHON

  windows:
    needs: choose
    runs-on: ${{ fromJSON(needs.choose.outputs.windows) }}
    timeout-minutes: 15
    steps:
      - name: Show Windows runner
        shell: powershell
        run: Write-Host "Runner=$env:RUNNER_NAME OS=$env:RUNNER_OS"

  linux:
    needs: choose
    runs-on: ${{ fromJSON(needs.choose.outputs.linux) }}
    timeout-minutes: 15
    steps:
      - name: Show Linux runner
        shell: bash
        run: printf 'Runner=%s OS=%s\n' "$RUNNER_NAME" "$RUNNER_OS"
```

このサンプルはcheckoutやビルドを行いません。既存ビルドの移植では、固定commit SHAを使うActions、`setup-*`、shell、architecture、toolchain、cache keyを明示して比較します。`windows-2022`/`ubuntu-24.04`は環境差を抑える例であり、最新hosted labelの主張ではありません。

## 5. 競合と手動復旧

判定後にPCが停止したり、別workflowがRunnerを使用したりすると、後続jobは待機・失敗し得ます。`timeout-minutes`はself-hosted待ちからの自動切替スイッチではありません。[S22](19_SOURCES.md#s22)

MVPは待ちを検出した利用者に、元の実行を確認して取消し、新しい`workflow_dispatch`を`force_hosted=true`で実行する方法を提示します。同じ実行のRe-runでinputを変えられると説明しません。副作用のあるdeployを二重実行しないよう、取消完了・実行済みstepを確認します。

## 6. 既存Workflow解析の仕様

MVPは固定labelを使う単純jobを解析し、説明と新規テンプレートを出します。後続のパッチ機能はYAML 1.2対応のCST/ASTを使い、コメント、`on`キー、式、既存`needs`、if条件、権限、job順序を保存します。

matrix、reusable workflow、dynamic runs-on、anchors、environment保護、複雑な条件は検出し、最初は自動変換を断ります。「全Workflow対応」とは表示しません。

書込み時は読取時のSHA-256を照合し、原本が変わっていれば差分を再作成します。API経由のPR作成やSecret更新は別権限・別同意であり、MVPに紛れ込ませません。

## 7. 必須テスト

falseのbusy値、missing/null/stringのbusy、offline、label不足、Runner ID相違、API認証失効、404、429、timeout、PR、手動別ref、public repo、hosted禁止、selector直後のPC停止、2workflow同時実行を含めます。

一覧を使う生成方式を後続で追加する場合はpaginationを完了させます。このサンプルは既知IDの個別取得なので一覧のpaginationには依存しません。[S24](19_SOURCES.md#s24)
