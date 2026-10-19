// Unicode scalar values: JSON Schema maxLength and Rust str::chars agree.
export const MAX_TASK_TITLE_CHARACTERS = 70;
export const taskTitleLength = (title: string) => Array.from(title).length;
export const validTaskTitle = (title: string) => Boolean(title.trim()) && taskTitleLength(title) <= MAX_TASK_TITLE_CHARACTERS;
