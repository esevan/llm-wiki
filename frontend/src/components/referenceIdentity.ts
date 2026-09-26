import type { ExactReferenceBinding } from "../types/taskWorkbench";
export const referenceKey = (reference: Pick<ExactReferenceBinding, "documentId" | "documentVersion" | "section">) => `${reference.documentId}\u0000${reference.documentVersion}\u0000${reference.section ?? ""}`;
