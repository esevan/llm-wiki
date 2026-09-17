import { useRef } from "react";
import type { InputImage } from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";
import type { useInputImage } from "./useInputImage";

export function InputImagePreview({ image }: { image: InputImage }) {
  const text = useTaskWorkbenchText();
  return <figure className="input-image-preview">
    <img src={`data:${image.mediaType};base64,${image.data}`} alt={image.name || text.attachedImage} />
    <figcaption>{image.name}</figcaption>
  </figure>;
}

export function InputImageAttachment({ attachment, disabled }: { attachment: ReturnType<typeof useInputImage>; disabled?: boolean }) {
  const text = useTaskWorkbenchText();
  const input = useRef<HTMLInputElement>(null);
  return <div className="input-image-attachment" aria-busy={attachment.reading}>
    <input data-control="input-image-file" ref={input} type="file" multiple hidden accept="image/png,image/jpeg,image/gif,image/webp" disabled={disabled || attachment.reading}
      aria-label={text.attachImage} onChange={event => { void attachment.select(Array.from(event.target.files ?? [])); event.target.value = ""; }} />
    <button data-control="input-image-choose" type="button" disabled={disabled || attachment.reading} onClick={() => input.current?.click()}>{attachment.reading ? "…" : text.attachImage}</button>
    <small>{text.imageHint}</small>
    {attachment.images.map((image, index) => <div key={index}><InputImagePreview image={image} />
      <button data-control="input-image-remove" type="button" disabled={disabled || attachment.reading} aria-label={`${text.removeImage}: ${image.name}`} onClick={() => attachment.remove(index)}>{text.removeImage}</button></div>)}
    {attachment.error && <p role="alert">{attachment.error}</p>}
  </div>;
}
