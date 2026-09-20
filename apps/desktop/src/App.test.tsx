import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { App } from "./App";

// globals を有効にしていないため、Testing Library の自動 cleanup は登録されない。
// 明示的に片付けないとテスト間で DOM が積み上がる。
afterEach(cleanup);

describe("App", () => {
  it("shows that it reads mock data when no data source is configured", () => {
    render(<App />);
    expect(screen.getByTestId("data-source").textContent).toBe("mock");
  });

  it("does not claim a shell connection when running outside Tauri", () => {
    render(<App />);
    expect(screen.getByTestId("shell-peer").textContent).toContain("未接続");
  });
});
