export function getMonitorPreviewLabel(
  monitorName: string | null | undefined,
): string | null {
  const normalizedName = monitorName?.trim();

  return normalizedName ? normalizedName : null;
}

export function getMonitorPreviewResolutionLabel(
  width: number,
  height: number,
): string {
  return `${width} x ${height}`;
}
