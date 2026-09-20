import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "src-tauri/target", "src-tauri/gen"] },
  js.configs.recommended,
  tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],

      // docs/15_DEVELOPER_GUIDE.md §5: any と未知 JSON の無検証 cast を避ける。
      "@typescript-eslint/no-explicit-any": "error",

      // AGENTS.md §5.1: UI から OS シェル・DB を直接叩く経路を作らない。
      "no-restricted-globals": [
        "error",
        { name: "eval", message: "任意コード実行を UI へ持ち込まない。" },
      ],
      "no-restricted-properties": [
        "error",
        {
          object: "window",
          property: "eval",
          message: "任意コード実行を UI へ持ち込まない。",
        },
      ],
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "@tauri-apps/plugin-shell",
              message:
                "UI から OS シェルを起動しない。操作は型付きの許可済み command を経由する（AGENTS.md §5.1）。",
            },
          ],
        },
      ],
    },
  },
);
