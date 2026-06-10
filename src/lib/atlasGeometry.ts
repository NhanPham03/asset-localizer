import type { AtlasInfo, AtlasRegion } from "../types";

export type Point = [number, number];

export interface DisplayRegion {
  name: string;
  rotated: boolean;
  corners: Point[];
  center: Point;
}

export interface ImageDisplayMetrics {
  scaleX: number;
  scaleY: number;
  offsetX: number;
  offsetY: number;
  renderWidth: number;
  renderHeight: number;
}

/** Uniform scale + offsets for an image using object-fit: contain in a box. */
export function imageDisplayMetrics(
  naturalWidth: number,
  naturalHeight: number,
  clientWidth: number,
  clientHeight: number,
): ImageDisplayMetrics {
  if (naturalWidth <= 0 || naturalHeight <= 0 || clientWidth <= 0 || clientHeight <= 0) {
    return {
      scaleX: 1,
      scaleY: 1,
      offsetX: 0,
      offsetY: 0,
      renderWidth: clientWidth,
      renderHeight: clientHeight,
    };
  }

  const imageRatio = naturalWidth / naturalHeight;
  const boxRatio = clientWidth / clientHeight;

  let renderWidth: number;
  let renderHeight: number;
  if (imageRatio > boxRatio) {
    renderWidth = clientWidth;
    renderHeight = clientWidth / imageRatio;
  } else {
    renderHeight = clientHeight;
    renderWidth = clientHeight * imageRatio;
  }

  return {
    scaleX: renderWidth / naturalWidth,
    scaleY: renderHeight / naturalHeight,
    offsetX: (clientWidth - renderWidth) / 2,
    offsetY: (clientHeight - renderHeight) / 2,
    renderWidth,
    renderHeight,
  };
}

/**
 * Atlas/plist rects in texture pixel space (same top-left origin as the PNG).
 * When the atlas page size differs from the loaded image, scale into image pixels.
 */
export function regionTextureRect(
  region: AtlasRegion,
  atlas: AtlasInfo | null | undefined,
  imageWidth: number,
  imageHeight: number,
): { x: number; y: number; width: number; height: number } {
  const texW = atlas?.texture_width ?? imageWidth;
  const texH = atlas?.texture_height ?? imageHeight;
  const sx = texW > 0 ? imageWidth / texW : 1;
  const sy = texH > 0 ? imageHeight / texH : 1;

  return {
    x: region.x * sx,
    y: region.y * sy,
    width: region.width * sx,
    height: region.height * sy,
  };
}

/** Map texture-pixel rect to canvas display coordinates (object-fit scaling only). */
export function regionToDisplay(
  region: AtlasRegion,
  atlas: AtlasInfo | null | undefined,
  imageWidth: number,
  imageHeight: number,
  scaleX: number,
  scaleY: number,
  offsetX = 0,
  offsetY = 0,
): DisplayRegion {
  const rect = regionTextureRect(region, atlas, imageWidth, imageHeight);
  const sx = rect.x * scaleX + offsetX;
  const sy = rect.y * scaleY + offsetY;
  const sw = rect.width * scaleX;
  const sh = rect.height * scaleY;

  const corners: Point[] = [
    [sx, sy],
    [sx + sw, sy],
    [sx + sw, sy + sh],
    [sx, sy + sh],
  ];

  return {
    name: region.name,
    rotated: region.rotated,
    corners,
    center: [sx + sw / 2, sy + sh / 2],
  };
}

export function pointInPolygon(px: number, py: number, corners: Point[]): boolean {
  let inside = false;
  for (let i = 0, j = corners.length - 1; i < corners.length; j = i++) {
    const [xi, yi] = corners[i];
    const [xj, yj] = corners[j];
    const intersect =
      yi > py !== yj > py &&
      px < ((xj - xi) * (py - yi)) / (yj - yi + Number.EPSILON) + xi;
    if (intersect) inside = !inside;
  }
  return inside;
}
