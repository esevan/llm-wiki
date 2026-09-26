import type {
  ReferenceWorkspace,
  WorkPreviewVersion,
} from "../../types/taskWorkbench";
export const fixtureVersion = (version = 1): WorkPreviewVersion => ({
  previewId: "preview",
  version,
  contentHash: `hash-${version}`,
  taskRevision: 1,
  contextRevision: 1,
  derivationKind: "generated",
  fields: {
    description: `Contract preview ${version}`,
    background: "Local approval conditions",
    goal: `Goal ${version}`,
    scope: "Contract",
    nonGoals: "Deployment",
    constraints: ["Approval first"],
    completionCriteria: ["Reviewed contract"],
    initialApproach: ["Check approval"],
    assumptions: [
      {
        id: "a",
        text: "Approver remains available",
        basis: "model_inference",
        sourceClaimIds: [],
      },
    ],
  },
  references: [
    {
      documentId: "approval",
      documentVersion: "source-v1",
      title: "Approval policy",
      section: "Approval",
      status: "current",
      path: "approval.md",
      excerpt: "Approval is required",
    },
    {
      documentId: "rollout",
      documentVersion: "source-v2",
      title: "Rollout policy",
      section: "Rollout",
      status: "historical",
      path: "rollout.md",
      excerpt: "Rollout is separate",
    },
  ],
});
export const fixtureWorkspace = (version = 1): ReferenceWorkspace => ({
  preview: {
    id: "preview",
    currentVersion: version,
    contextRevision: 1,
    current: fixtureVersion(version),
    versions: Array.from({ length: version }, (_, index) => ({
      version: index + 1,
      derivationKind: "generated",
    })),
  },
  generation: { id: "job", status: "completed" },
  retrievalOutcome: "results",
  investigations: [],
  findings: [],
});
