import type { ApplicationClient } from '../types/application';
import { TauriApplicationClient } from './tauriApplicationClient';

export const createApplicationClient = (): ApplicationClient => new TauriApplicationClient();
