import { useRef, useState, type ClipboardEvent } from "react";
import type { InputImage } from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

export function useInputImage(initial?: InputImage) {
  const text = useTaskWorkbenchText();
  const [image, setImage] = useState(initial);
  const [reading, setReading] = useState(false);
  const [error, setError] = useState("");
  const readingRef = useRef(false);
  const select = (file?: File) => {
    if (!file || readingRef.current) return;
    if (!["image/png", "image/jpeg", "image/gif", "image/webp"].includes(file.type) || file.size > 10 * 1024 * 1024 || !file.size) {
      setError(text.imageError);
      return;
    }
    readingRef.current = true;
    setReading(true);
    setError("");
    const reader = new FileReader();
    reader.onload = () => setImage({ name: file.name || "image", mediaType: file.type, data: String(reader.result).split(",")[1] });
    reader.onerror = () => setError(text.imageReadError);
    reader.onloadend = () => { readingRef.current = false; setReading(false); };
    reader.readAsDataURL(file);
  };
  const paste = (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const file = [...event.clipboardData.items].find(item => item.type.startsWith("image/"))?.getAsFile()
      ?? Array.from(event.clipboardData.files ?? []).find(file => file.type.startsWith("image/"));
    if (file) { event.preventDefault(); select(file); }
  };
  const clear = () => { setImage(undefined); setError(""); };
  return { image, reading, error, select, paste, clear, restore: setImage };
}
