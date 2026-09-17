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
    act(() => { void result.current.select(new File(["<svg/>"], "vector.svg", { type: "image/svg+xml" })); });
    expect(result.current.error).not.toBe("");
    expect(result.current.image?.name).toBe("shot.png");
    act(() => result.current.clear());
    expect(result.current.image).toBeUndefined();
    expect(result.current.error).toBe("");
  });

  it("appends multiple selections and clipboard images in order and removes only the chosen image", async () => {
    const { result } = renderHook(() => useInputImage());
    const file = (name: string) => new File(["bytes"], name, { type: "image/png" });
    act(() => { void result.current.select([file("one.png"), file("two.png")]); });
    await waitFor(() => expect(result.current.images).toHaveLength(2));
    const files = [file("three.png"), file("four.png")];
    act(() => result.current.paste({ preventDefault: vi.fn(), clipboardData: {
      items: files.map(file => ({ type: file.type, getAsFile: () => file })), files,
    } } as unknown as ClipboardEvent<HTMLTextAreaElement>));
    await waitFor(() => expect(result.current.images.map(image => image.name)).toEqual(["one.png", "two.png", "three.png", "four.png"]));
    act(() => result.current.remove(1));
    expect(result.current.images.map(image => image.name)).toEqual(["one.png", "three.png", "four.png"]);
    act(() => { void result.current.select([file("valid.png"), new File(["bad"], "bad.svg", { type: "image/svg+xml" })]); });
    expect(result.current.images).toHaveLength(3);
    expect(result.current.error).not.toBe("");
  });

  it("preserves ordinary text paste and validates the size before reading", () => {
    const { result } = renderHook(() => useInputImage());
    const event = { preventDefault: vi.fn(), clipboardData: { items: [], files: [] } };
    act(() => result.current.paste(event as unknown as ClipboardEvent<HTMLTextAreaElement>));
    expect(event.preventDefault).not.toHaveBeenCalled();
    const file = new File(["image"], "large.png", { type: "image/png" });
    Object.defineProperty(file, "size", { value: 10 * 1024 * 1024 + 1 });
    act(() => { void result.current.select(file); });
    expect(result.current.error).not.toBe("");
    expect(result.current.reading).toBe(false);
    expect(result.current.image).toBeUndefined();
  });
});
