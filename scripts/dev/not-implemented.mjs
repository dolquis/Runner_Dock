// docs/15_DEVELOPER_GUIDE.md「初期化後に定義する開発コマンド」のうち、LF-001 の
// 範囲外で実体を持たないものを表す。
//
// exit 0 のダミーにすると「検査して通った」と区別できなくなるため、非 0 で
// 終了し、どのタスクで実装するかを出力する。
const [, , command, owner] = process.argv;

process.stderr.write(
  [
    `${command}: このコマンドの実体は LF-001 の範囲外です。`,
    `実装は ${owner} で行います（docs/14_BACKLOG.md）。`,
    "検査が通ったものとして扱わないでください。",
    "",
  ].join("\n"),
);
process.exit(1);
