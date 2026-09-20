import { describe, expect, it } from "vitest";

import { UnknownDataSourceError, resolveDataSource } from "./data-source";

describe("resolveDataSource", () => {
  it("defaults to mock when the environment variable is unset", () => {
    expect(resolveDataSource(undefined)).toBe("mock");
    expect(resolveDataSource("")).toBe("mock");
    expect(resolveDataSource("  ")).toBe("mock");
  });

  it("selects live only when it is requested explicitly", () => {
    expect(resolveDataSource("live")).toBe("live");
    expect(resolveDataSource("mock")).toBe("mock");
  });

  it("throws instead of falling back for an unrecognized value", () => {
    expect(() => resolveDataSource("production")).toThrow(UnknownDataSourceError);
    expect(() => resolveDataSource("LIVE")).toThrow(UnknownDataSourceError);
  });

  it("keeps the rejected value for diagnostics", () => {
    try {
      resolveDataSource("liv");
      expect.unreachable("an unknown data source must not resolve");
    } catch (error) {
      expect(error).toBeInstanceOf(UnknownDataSourceError);
      expect((error as UnknownDataSourceError).received).toBe("liv");
    }
  });
});
