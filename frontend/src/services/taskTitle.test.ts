import { describe, expect, it } from "vitest";
import { taskTitleLength, validTaskTitle } from "./taskTitle";
describe("Task title Unicode contract", () => {
  it("counts Korean, emoji, combining marks and ZWJ like JSON Schema and Rust", () => {
    for (const c of ["가", "😀", "a"]) {
      expect(validTaskTitle(c.repeat(70))).toBe(true);
      expect(validTaskTitle(c.repeat(71))).toBe(false);
    }
    expect(taskTitleLength("e\u0301")).toBe(2);
    expect(taskTitleLength("👨‍👩‍👧‍👦")).toBe(7);
    expect(validTaskTitle("👨‍👩‍👧‍👦".repeat(10))).toBe(true);
    expect(validTaskTitle(" ")).toBe(false);
  });
});
