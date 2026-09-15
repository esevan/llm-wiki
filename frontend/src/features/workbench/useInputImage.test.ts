import { act, renderHook, waitFor } from "@testing-library/react";
import type { ClipboardEvent } from "react";
import { describe, expect, it, vi } from "vitest";
import { useInputImage } from "./useInputImage";

describe("input image drafts", () => {
  it("reads clipboard images, rejects unsupported replacements, and clears explicitly", async () => {
    const { result } = renderHook(() => useInputImage());
    const file = new File([new Uint8Array([137,80,78,71,13,10,26,10])], "shot.png", { type: "image/png" });
    const event = { preventDefault: vi.fn(), clipboardData: { items: [{ type: file.type, getAsFile: () => file }], files: [] } };
    act(() => result.current.paste(event as unknown as ClipboardEvent<HTMLTextAreaElement>));
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await waitFor(() => expect(result.current.reading).toBe(false));
    expect(result.current.image).toEqual({ name: "shot.png", mediaType: "image/png", data: "iVBORw0KGgo=" });
    act(() => result.current.select(new File(["<svg/>"], "vector.svg", { type: "image/svg+xml" })));
    expect(result.current.error).not.toBe("");
    expect(result.current.image?.name).toBe("shot.png");
    act(() => result.current.clear());
    expect(result.current.image).toBeUndefined();
    expect(result.current.error).toBe("");
  });

  it("preserves ordinary text paste and validates the size before reading", () => {
    const { result } = renderHook(() => useInputImage());
    const event = { preventDefault: vi.fn(), clipboardData: { items: [], files: [] } };
    act(() => result.current.paste(event as unknown as ClipboardEvent<HTMLTextAreaElement>));
    expect(event.preventDefault).not.toHaveBeenCalled();
    const file = new File(["image"], "large.png", { type: "image/png" });
    Object.defineProperty(file, "size", { value: 10 * 1024 * 1024 + 1 });
    act(() => result.current.select(file));
    expect(result.current.error).not.toBe("");
    expect(result.current.reading).toBe(false);
    expect(result.current.image).toBeUndefined();
  });
});
