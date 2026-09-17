import { useRef, useState, type ClipboardEvent } from "react";
import type { InputImage } from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

export function useInputImage(initial: InputImage[] = []) {
  const text = useTaskWorkbenchText();
  const [images, setImages] = useState(initial);
  const [reading, setReading] = useState(false);
  const [error, setError] = useState("");
  const readingRef = useRef(false);
  const select = async (selection?: File | File[]) => {
    const files = selection instanceof File ? [selection] : selection ?? [];
    if (!files.length || readingRef.current) return;
    if (files.some(file => !["image/png", "image/jpeg", "image/gif", "image/webp"].includes(file.type) || file.size > 10 * 1024 * 1024 || !file.size)) {
      setError(text.imageError);
      return;
    }
    readingRef.current = true;
    setReading(true);
    setError("");
    try {
      const added = await Promise.all(files.map(file => new Promise<InputImage>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve({ name: file.name || "image", mediaType: file.type, data: String(reader.result).split(",")[1] });
        reader.onerror = reader.onabort = () => reject(new Error(text.imageReadError));
        reader.readAsDataURL(file);
      })));
      setImages(current => [...current, ...added]);
    } catch { setError(text.imageReadError); }
    finally { readingRef.current = false; setReading(false); }
  };
  const paste = (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const items = Array.from(event.clipboardData.items).filter(item => item.type.startsWith("image/"))
      .map(item => item.getAsFile()).filter((file): file is File => Boolean(file));
    const files = items.length ? items : Array.from(event.clipboardData.files ?? []).filter(file => file.type.startsWith("image/"));
    if (files.length) { event.preventDefault(); void select(files); }
  };
  const clear = () => { setImages([]); setError(""); };
  const remove = (index: number) => setImages(current => current.filter((_, i) => i !== index));
  return { images, image: images[0], reading, error, select, paste, clear, remove, restore: setImages };
}
