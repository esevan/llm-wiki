import sidebar from '../app/Sidebar.tsx?raw';
import chatTracking from '../features/chat/ChatTrackingSurface.tsx?raw';
import trackingCards from '../features/chat/WorkTrackingCards.tsx?raw';
import compass from '../features/compass/CompassView.tsx?raw';
import intro from '../features/first-run-intro/FirstRunIntro.tsx?raw';
import overlays from '../features/overlays/OverlayLayer.tsx?raw';
import search from '../features/search/SearchView.tsx?raw';
import mcp from '../features/settings/McpConnections.tsx?raw';
import settings from '../features/settings/SettingsView.tsx?raw';
import migration from '../features/vault-setup/MigrationRecoveryView.tsx?raw';
import vaultSetup from '../features/vault-setup/VaultSetupView.tsx?raw';
import conflictReview from '../features/workbench/ConflictReviewPanel.tsx?raw';
import refinement from '../features/workbench/RefinementPanel.tsx?raw';
import deleteDialog from '../features/workbench/DeleteItemDialog.tsx?raw';
import taskDetail from '../features/workbench/TaskDetail.tsx?raw';
import workbench from '../features/workbench/WorkbenchView.tsx?raw';
import archiveRuntime from '../../public/runtime/archive.js?raw';
import completedRuntime from '../../public/runtime/completed-workspace.js?raw';
import conflictsRuntime from '../../public/runtime/conflicts.js?raw';
import exploreRuntime from '../../public/runtime/explore.js?raw';
import foundationRuntime from '../../public/runtime/foundation.js?raw';
import jobsRuntime from '../../public/runtime/jobs.js?raw';
import manualRuntime from '../../public/runtime/manual.js?raw';
import searchRuntime from '../../public/runtime/search-settings.js?raw';
import solutionRuntime from '../../public/runtime/solution-work.js?raw';
import transitionsRuntime from '../../public/runtime/transitions.js?raw';
import trackingRuntime from '../../public/runtime/work-tracking.js?raw';
import workbenchRuntime from '../../public/runtime/workbench.js?raw';

import { InteractionCoverage } from './interactionCoverage';
import { taskInteractiveManifest } from './interactionCoverageManifest';

/** Every React source with a shipping control plus every runtime loaded by index.html. */
export const productionInteractiveSources = new Map<string, string>([
  ['frontend/src/app/Sidebar.tsx', sidebar],
  ['frontend/src/features/chat/ChatTrackingSurface.tsx', chatTracking],
  ['frontend/src/features/chat/WorkTrackingCards.tsx', trackingCards],
  ['frontend/src/features/compass/CompassView.tsx', compass],
  ['frontend/src/features/first-run-intro/FirstRunIntro.tsx', intro],
  ['frontend/src/features/overlays/OverlayLayer.tsx', overlays],
  ['frontend/src/features/search/SearchView.tsx', search],
  ['frontend/src/features/settings/McpConnections.tsx', mcp],
  ['frontend/src/features/settings/SettingsView.tsx', settings],
  ['frontend/src/features/vault-setup/MigrationRecoveryView.tsx', migration],
  ['frontend/src/features/vault-setup/VaultSetupView.tsx', vaultSetup],
  ['frontend/src/features/workbench/ConflictReviewPanel.tsx', conflictReview],
  ['frontend/src/features/workbench/RefinementPanel.tsx', refinement],
  ['frontend/src/features/workbench/TaskDetail.tsx', taskDetail],
  ['frontend/src/features/workbench/DeleteItemDialog.tsx', deleteDialog],
  ['frontend/src/features/workbench/WorkbenchView.tsx', workbench],
  ['frontend/public/runtime/archive.js', archiveRuntime],
  ['frontend/public/runtime/completed-workspace.js', completedRuntime],
  ['frontend/public/runtime/conflicts.js', conflictsRuntime],
  ['frontend/public/runtime/explore.js', exploreRuntime],
  ['frontend/public/runtime/foundation.js', foundationRuntime],
  ['frontend/public/runtime/jobs.js', jobsRuntime],
  ['frontend/public/runtime/manual.js', manualRuntime],
  ['frontend/public/runtime/search-settings.js', searchRuntime],
  ['frontend/public/runtime/solution-work.js', solutionRuntime],
  ['frontend/public/runtime/transitions.js', transitionsRuntime],
  ['frontend/public/runtime/work-tracking.js', trackingRuntime],
  ['frontend/public/runtime/workbench.js', workbenchRuntime],
]);

export function createInteractionCoverage() {
  return new InteractionCoverage(taskInteractiveManifest, productionInteractiveSources);
}
