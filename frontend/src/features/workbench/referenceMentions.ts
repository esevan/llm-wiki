import type { DocumentMention } from "../../types/taskWorkbench";

export type PositionedMention = DocumentMention & {
  start: number;
  end: number;
};

// Old drafts contain exact bindings but no offsets. Match their ordered visible
// tokens once; newly pasted matching text never creates an additional binding.
export function positionMentions(
  text: string,
  mentions: DocumentMention[],
): PositionedMention[] {
  const occupied: Array<[number, number]> = [];
  return mentions.flatMap((mention) => {
    const token = `@${mention.displayLabel}`;
    let start = mention.start;
    if (start === undefined || text.slice(start, mention.end) !== token) {
      start = text.indexOf(token);
      while (
        start >= 0 &&
        occupied.some(
          ([left, right]) => start! < right && start! + token.length > left,
        )
      )
        start = text.indexOf(token, start + 1);
    }
    if (
      start < 0 ||
      occupied.some(
        ([left, right]) => start! < right && start! + token.length > left,
      )
    )
      return [];
    const end = start + token.length;
    occupied.push([start, end]);
    return [{ ...mention, start, end }];
  });
}

export function reconcileMentionEdit(
  before: string,
  after: string,
  mentions: DocumentMention[],
): PositionedMention[] {
  let start = 0;
  while (
    start < before.length &&
    start < after.length &&
    before[start] === after[start]
  )
    start += 1;
  let suffix = 0;
  while (
    suffix < before.length - start &&
    suffix < after.length - start &&
    before[before.length - 1 - suffix] === after[after.length - 1 - suffix]
  )
    suffix += 1;
  const oldEnd = before.length - suffix;
  const delta = after.length - before.length;
  return positionMentions(before, mentions).flatMap((mention) => {
    if (mention.end <= start) return [mention];
    if (mention.start >= oldEnd)
      return [
        { ...mention, start: mention.start + delta, end: mention.end + delta },
      ];
    // Editing within a token intentionally turns it into readable plain text.
    return [];
  });
}

export function deleteMentionRange(
  text: string,
  mentions: DocumentMention[],
  start: number,
  end: number,
  key: "Backspace" | "Delete",
) {
  let left =
    start === end && key === "Backspace" ? Math.max(0, start - 1) : start;
  let right =
    start === end && key === "Delete" ? Math.min(text.length, end + 1) : end;
  const overlapping = positionMentions(text, mentions).filter(
    (mention) => mention.start < right && mention.end > left,
  );
  if (!overlapping.length) return undefined;
  for (const mention of overlapping) {
    left = Math.min(left, mention.start);
    right = Math.max(right, mention.end);
  }
  const value = text.slice(0, left) + text.slice(right);
  return {
    value,
    mentions: reconcileMentionEdit(text, value, mentions),
    caret: left,
  };
}

export function removeMention(
  text: string,
  mentions: DocumentMention[],
  id: string,
) {
  const target = positionMentions(text, mentions).find(
    (mention) => mention.mentionId === id,
  );
  if (!target)
    return {
      value: text,
      mentions: mentions.filter((mention) => mention.mentionId !== id),
      caret: text.length,
    };
  return deleteMentionRange(
    text,
    mentions,
    target.start,
    target.end,
    "Delete",
  )!;
}
