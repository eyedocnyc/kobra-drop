export function newerStable(current: string, candidate: string): boolean;
export function canInstall(state: {
  converting: boolean;
  pending: boolean;
  selecting: boolean;
  installing: boolean;
  resultShown: boolean;
}): boolean;
