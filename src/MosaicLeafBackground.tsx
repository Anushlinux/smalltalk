import { useEffect, useRef } from "react";
import mosaicLeafSource from "./assets/mosaic-leaf.webp";

type RenderMode =
  | "characters"
  | "dither"
  | "mosaic"
  | "pixel"
  | "dots"
  | "cross"
  | "diamond"
  | "voxel"
  | "lego"
  | "mixed"
  | "lines"
  | "diagonal"
  | "braille"
  | "disco"
  | "hexdump"
  | "matrix"
  | "rings"
  | "hearts"
  | "stars"
  | "hexagons"
  | "triangles"
  | "bubbles"
  | "hatch"
  | "contour"
  | "halfblocks";

type AnimationStyle = "wave" | "pulse" | "shimmer" | "ripple" | "flicker";
type BackgroundMode = "blurred" | "solid" | "original" | "none";
type BlurType = "off" | "gaussian" | "motion" | "tilt" | "lens" | "progressive";

type EffectValue = {
  enabled: boolean;
  intensity: number;
};

type MosaicParameters = {
  renderMode: RenderMode;
  bgMode: BackgroundMode;
  bgBlur: number;
  bgOpacity: number;
  cellSize: number;
  coverage: number;
  invert: boolean;
  styleBlend: GlobalCompositeOperation;
  charSet: "standard" | "blocks" | "minimal" | "binary";
  customChars: string;
  brightness: number;
  contrast: number;
  edgeEmphasis: number;
  density: number;
  toneCurve: Array<{ x: number; y: number }>;
  tint: string;
  tintOpacity: number;
  overlayBlend: GlobalCompositeOperation;
  saturation: number;
  grayscale: number;
  blurType: BlurType;
  blurAmount: number;
  blurAngle: number;
  directionalBothSides: boolean;
  tiltFocus: number;
  tiltPosition: number;
  tiltFeather: number;
  lensFocus: number;
  blurCenterX: number;
  blurCenterY: number;
  progressivePosition: number;
  progressiveReverse: boolean;
  pfx: Record<
    | "vignette"
    | "scanLines"
    | "chromatic"
    | "bloom"
    | "filmGrain"
    | "glitch"
    | "pixelate"
    | "halftone"
    | "filmDust",
    EffectValue
  >;
  animated: boolean;
  animStyle: AnimationStyle;
  animSpeed: EffectValue;
  animIntensity: EffectValue;
  lights: {
    enabled: boolean;
    points: Array<{ x: number; y: number; radius: number; intensity: number }>;
  };
  mask: {
    enabled: boolean;
    invert: boolean;
    dataUrl: string | null;
  };
};

type SampledCell = {
  col: number;
  row: number;
  x: number;
  y: number;
  r: number;
  g: number;
  b: number;
  luma: number;
  edge: number;
};

type AnimationValue = {
  alpha: number;
  scale: number;
  offsetX: number;
  offsetY: number;
};

type RenderBuffers = {
  frame: HTMLCanvasElement;
  primitives: HTMLCanvasElement;
  adjusted: HTMLCanvasElement;
  bloom: HTMLCanvasElement;
};

const MOSAIC_LEAF_PARAMETERS: MosaicParameters = {
  renderMode: "mosaic",
  bgMode: "solid",
  bgBlur: 12,
  bgOpacity: 90,
  cellSize: 11,
  coverage: 100,
  invert: false,
  styleBlend: "lighter",
  charSet: "standard",
  customChars: "",
  brightness: 12,
  contrast: 115,
  edgeEmphasis: 0,
  density: 0,
  toneCurve: [{ x: 0, y: 0 }, { x: 1, y: 1 }],
  tint: "#3ca6ff",
  tintOpacity: 0,
  overlayBlend: "multiply",
  saturation: 100,
  grayscale: 0,
  blurType: "off",
  blurAmount: 35,
  blurAngle: 0,
  directionalBothSides: false,
  tiltFocus: 35,
  tiltPosition: 50,
  tiltFeather: 15,
  lensFocus: 40,
  blurCenterX: 50,
  blurCenterY: 50,
  progressivePosition: 55,
  progressiveReverse: false,
  pfx: {
    vignette: { enabled: true, intensity: 38 },
    scanLines: { enabled: false, intensity: 40 },
    chromatic: { enabled: false, intensity: 15 },
    bloom: { enabled: true, intensity: 25 },
    filmGrain: { enabled: false, intensity: 30 },
    glitch: { enabled: false, intensity: 20 },
    pixelate: { enabled: false, intensity: 15 },
    halftone: { enabled: false, intensity: 20 },
    filmDust: { enabled: false, intensity: 20 },
  },
  animated: true,
  animStyle: "wave",
  animSpeed: { enabled: true, intensity: 100 },
  animIntensity: { enabled: true, intensity: 60 },
  lights: { enabled: false, points: [] },
  mask: { enabled: false, invert: false, dataUrl: null },
};

const CHARACTER_SETS = {
  standard: " .:-=+*#%@",
  blocks: " ░▒▓█",
  minimal: " ·+×#",
  binary: " 01",
};

const BAYER_4 = [
  0, 8, 2, 10,
  12, 4, 14, 6,
  3, 11, 1, 9,
  15, 7, 13, 5,
];

function clamp(value: number, minimum = 0, maximum = 1) {
  return Math.min(maximum, Math.max(minimum, value));
}

function hash2d(x: number, y: number, seed = 0) {
  const value = Math.sin(x * 127.1 + y * 311.7 + seed * 74.7) * 43758.5453;
  return value - Math.floor(value);
}

function toneCurveValue(value: number, points: MosaicParameters["toneCurve"]) {
  const sorted = [...points].sort((a, b) => a.x - b.x);
  if (sorted.length === 0) return value;
  if (value <= sorted[0].x) return sorted[0].y;
  for (let index = 1; index < sorted.length; index += 1) {
    const left = sorted[index - 1];
    const right = sorted[index];
    if (value <= right.x) {
      const progress = (value - left.x) / Math.max(0.0001, right.x - left.x);
      return left.y + (right.y - left.y) * progress;
    }
  }
  return sorted[sorted.length - 1].y;
}

function drawImageCover(
  context: CanvasRenderingContext2D,
  image: CanvasImageSource,
  sourceWidth: number,
  sourceHeight: number,
  width: number,
  height: number,
  horizontalOffset = 0,
) {
  const scale = Math.max(width / sourceWidth, height / sourceHeight);
  const drawWidth = sourceWidth * scale;
  const drawHeight = sourceHeight * scale;
  context.drawImage(
    image,
    (width - drawWidth) / 2 + horizontalOffset,
    (height - drawHeight) / 2,
    drawWidth,
    drawHeight,
  );
}

function sampleCells(
  photo: HTMLCanvasElement,
  width: number,
  height: number,
  cellSize: number,
) {
  const context = photo.getContext("2d", { willReadFrequently: true });
  if (!context) return [];
  const pixels = context.getImageData(0, 0, width, height).data;
  const cells: SampledCell[] = [];
  const columns = Math.ceil(width / cellSize);
  const rows = Math.ceil(height / cellSize);

  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < columns; col += 1) {
      const startX = col * cellSize;
      const startY = row * cellSize;
      const endX = Math.min(width, startX + cellSize);
      const endY = Math.min(height, startY + cellSize);
      let red = 0;
      let green = 0;
      let blue = 0;
      let alpha = 0;
      let count = 0;

      for (let y = startY; y < endY; y += 2) {
        for (let x = startX; x < endX; x += 2) {
          const offset = (y * width + x) * 4;
          red += pixels[offset];
          green += pixels[offset + 1];
          blue += pixels[offset + 2];
          alpha += pixels[offset + 3];
          count += 1;
        }
      }

      const alphaScale = count > 0 ? alpha / count / 255 : 0;
      const r = count > 0 ? red / count * alphaScale : 0;
      const g = count > 0 ? green / count * alphaScale : 0;
      const b = count > 0 ? blue / count * alphaScale : 0;
      cells.push({
        col,
        row,
        x: startX,
        y: startY,
        r,
        g,
        b,
        luma: (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255,
        edge: 0,
      });
    }
  }

  const at = (col: number, row: number) => {
    const safeCol = Math.max(0, Math.min(columns - 1, col));
    const safeRow = Math.max(0, Math.min(rows - 1, row));
    return cells[safeRow * columns + safeCol].luma;
  };
  cells.forEach((cell) => {
    const horizontal = at(cell.col + 1, cell.row) - at(cell.col - 1, cell.row);
    const vertical = at(cell.col, cell.row + 1) - at(cell.col, cell.row - 1);
    cell.edge = clamp(Math.hypot(horizontal, vertical) * 1.8);
  });
  return cells;
}

function animationValue(
  cell: SampledCell,
  time: number,
  width: number,
  height: number,
  parameters: MosaicParameters,
): AnimationValue {
  if (!parameters.animated || !parameters.animIntensity.enabled) {
    return { alpha: 1, scale: 1, offsetX: 0, offsetY: 0 };
  }
  const speed = parameters.animSpeed.enabled
    ? 0.45 + parameters.animSpeed.intensity / 100 * 1.35
    : 0;
  const intensity = parameters.animIntensity.intensity / 100;
  const seconds = time / 1000;
  let wave = 0;

  if (parameters.animStyle === "pulse") {
    wave = Math.sin(seconds * speed * 2.2);
  } else if (parameters.animStyle === "shimmer") {
    const position = (seconds * speed * 0.28) % 1.7 - 0.35;
    const cellPosition = (cell.x / Math.max(1, width) + cell.y / Math.max(1, height) * 0.24);
    wave = Math.exp(-Math.pow((cellPosition - position) * 7, 2)) * 2 - 0.45;
  } else if (parameters.animStyle === "ripple") {
    const distance = Math.hypot(cell.x - width * 0.62, cell.y - height * 0.52);
    wave = Math.sin(distance * 0.075 - seconds * speed * 3.2);
  } else if (parameters.animStyle === "flicker") {
    wave = hash2d(cell.col, cell.row, Math.floor(seconds * speed * 12)) * 2 - 1;
  } else {
    wave = Math.sin(cell.col * 0.31 + cell.row * 0.19 - seconds * speed * 2.15);
  }

  return {
    alpha: clamp(0.9 + wave * 0.12 * intensity, 0.58, 1),
    scale: 1 + wave * 0.07 * intensity,
    offsetX: Math.cos(cell.row * 0.25 - seconds * speed) * 0.42 * intensity,
    offsetY: wave * 1.35 * intensity,
  };
}

function roundedRect(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) {
  const safeRadius = Math.min(radius, width / 2, height / 2);
  context.beginPath();
  context.roundRect(x, y, width, height, safeRadius);
}

function drawStar(
  context: CanvasRenderingContext2D,
  centerX: number,
  centerY: number,
  outerRadius: number,
  innerRadius: number,
) {
  context.beginPath();
  for (let point = 0; point < 10; point += 1) {
    const radius = point % 2 === 0 ? outerRadius : innerRadius;
    const angle = -Math.PI / 2 + point * Math.PI / 5;
    const x = centerX + Math.cos(angle) * radius;
    const y = centerY + Math.sin(angle) * radius;
    if (point === 0) context.moveTo(x, y);
    else context.lineTo(x, y);
  }
  context.closePath();
}

function drawPrimitive(
  context: CanvasRenderingContext2D,
  cell: SampledCell,
  parameters: MosaicParameters,
  time: number,
  width: number,
  height: number,
) {
  if (hash2d(cell.col, cell.row) * 100 > parameters.coverage) return;
  const cellSize = parameters.cellSize;
  const animation = animationValue(cell, time, width, height, parameters);
  const edgeBoost = cell.edge * parameters.edgeEmphasis / 100;
  const rawLuma = parameters.invert ? 1 - cell.luma : cell.luma;
  const luma = clamp(toneCurveValue(clamp(rawLuma + edgeBoost), parameters.toneCurve));
  const densityScale = 1 + parameters.density / 100 * 0.35;
  const shapeSize = Math.max(1, cellSize * clamp((0.64 + luma * 0.42) * densityScale, 0.18, 1.18));
  const centerX = cell.x + cellSize / 2 + animation.offsetX;
  const centerY = cell.y + cellSize / 2 + animation.offsetY;
  const size = shapeSize * animation.scale;
  const color = `rgb(${Math.round(cell.r)} ${Math.round(cell.g)} ${Math.round(cell.b)})`;
  const brightColor = `rgb(${Math.min(255, Math.round(cell.r * 1.18 + 4))} ${Math.min(255, Math.round(cell.g * 1.18 + 4))} ${Math.min(255, Math.round(cell.b * 1.18 + 4))})`;
  let mode = parameters.renderMode;
  if (mode === "mixed") {
    mode = luma > 0.66 ? "characters" : luma > 0.32 ? "dots" : "diagonal";
  }

  context.save();
  context.globalAlpha = animation.alpha;
  context.fillStyle = color;
  context.strokeStyle = brightColor;
  context.lineWidth = Math.max(0.7, cellSize * 0.09);
  context.lineCap = "round";
  context.lineJoin = "round";

  if (mode === "mosaic" || mode === "pixel") {
    const gap = mode === "mosaic" ? Math.max(0.55, cellSize * 0.055) : 0;
    const x = centerX - cellSize / 2 + gap;
    const y = centerY - cellSize / 2 + gap;
    const tileSize = Math.max(1, cellSize - gap * 2);
    if (mode === "mosaic") {
      roundedRect(context, x, y, tileSize, tileSize, Math.max(0.8, cellSize * 0.11));
      context.fill();
    } else {
      context.fillRect(x, y, tileSize, tileSize);
    }
  } else if (mode === "dither") {
    const threshold = (BAYER_4[(cell.row % 4) * 4 + cell.col % 4] + 0.5) / 16;
    if (luma >= threshold) context.fillRect(cell.x, cell.y, cellSize, cellSize);
  } else if (mode === "dots" || mode === "disco" || mode === "bubbles") {
    context.beginPath();
    context.arc(centerX, centerY, Math.max(0.7, size * (mode === "bubbles" ? 0.43 : 0.36)), 0, Math.PI * 2);
    if (mode === "bubbles") {
      context.globalAlpha *= 0.38 + luma * 0.46;
      context.stroke();
    } else {
      if (mode === "disco") context.shadowColor = brightColor;
      if (mode === "disco") context.shadowBlur = size * 0.45;
      context.fill();
    }
  } else if (mode === "rings") {
    context.beginPath();
    context.arc(centerX, centerY, Math.max(1, size * 0.38), 0, Math.PI * 2);
    context.globalAlpha *= 0.45 + luma * 0.55;
    context.stroke();
  } else if (mode === "cross") {
    const arm = size * 0.4;
    context.beginPath();
    context.moveTo(centerX - arm, centerY);
    context.lineTo(centerX + arm, centerY);
    context.moveTo(centerX, centerY - arm);
    context.lineTo(centerX, centerY + arm);
    context.stroke();
  } else if (mode === "diamond") {
    context.beginPath();
    context.moveTo(centerX, centerY - size * 0.48);
    context.lineTo(centerX + size * 0.48, centerY);
    context.lineTo(centerX, centerY + size * 0.48);
    context.lineTo(centerX - size * 0.48, centerY);
    context.closePath();
    context.fill();
  } else if (mode === "lines" || mode === "diagonal") {
    const half = size * 0.43;
    context.beginPath();
    if (mode === "lines") {
      context.moveTo(centerX - half, centerY);
      context.lineTo(centerX + half, centerY);
    } else {
      const flip = (cell.col + cell.row) % 2 === 0 ? 1 : -1;
      context.moveTo(centerX - half, centerY + half * flip);
      context.lineTo(centerX + half, centerY - half * flip);
    }
    context.stroke();
  } else if (mode === "hatch") {
    const half = size * 0.42;
    context.globalAlpha *= 0.35 + luma * 0.65;
    context.beginPath();
    context.moveTo(centerX - half, centerY + half);
    context.lineTo(centerX + half, centerY - half);
    if (luma > 0.42) {
      context.moveTo(centerX - half, centerY - half);
      context.lineTo(centerX + half, centerY + half);
    }
    context.stroke();
  } else if (mode === "contour") {
    const level = Math.round(luma * 5) / 5;
    const offset = (level - 0.5) * cellSize;
    context.beginPath();
    context.moveTo(cell.x, centerY + offset * 0.42);
    context.bezierCurveTo(
      cell.x + cellSize * 0.3,
      centerY - offset * 0.36,
      cell.x + cellSize * 0.72,
      centerY + offset * 0.3,
      cell.x + cellSize,
      centerY - offset * 0.26,
    );
    context.globalAlpha *= 0.38 + cell.edge * 0.62;
    context.stroke();
  } else if (mode === "hexagons") {
    context.beginPath();
    for (let side = 0; side < 6; side += 1) {
      const angle = Math.PI / 6 + side * Math.PI / 3;
      const x = centerX + Math.cos(angle) * size * 0.48;
      const y = centerY + Math.sin(angle) * size * 0.48;
      if (side === 0) context.moveTo(x, y);
      else context.lineTo(x, y);
    }
    context.closePath();
    context.globalAlpha *= 0.62 + luma * 0.38;
    context.stroke();
  } else if (mode === "triangles") {
    context.beginPath();
    if ((cell.row + cell.col) % 2 === 0) {
      context.moveTo(cell.x, cell.y + cellSize);
      context.lineTo(cell.x + cellSize, cell.y);
      context.lineTo(cell.x + cellSize, cell.y + cellSize);
    } else {
      context.moveTo(cell.x, cell.y);
      context.lineTo(cell.x + cellSize, cell.y);
      context.lineTo(cell.x, cell.y + cellSize);
    }
    context.closePath();
    context.fill();
  } else if (mode === "halfblocks") {
    const topLuma = clamp(luma + cell.edge * 0.18);
    context.globalAlpha *= 0.56 + topLuma * 0.44;
    context.fillRect(cell.x, cell.y, cellSize, cellSize / 2);
    context.globalAlpha *= 0.72;
    context.fillRect(cell.x, cell.y + cellSize / 2, cellSize, cellSize / 2);
  } else if (mode === "lego") {
    roundedRect(
      context,
      centerX - size * 0.45,
      centerY - size * 0.42,
      size * 0.9,
      size * 0.84,
      size * 0.16,
    );
    context.fill();
    context.fillStyle = brightColor;
    context.beginPath();
    context.arc(centerX, centerY - size * 0.08, size * 0.18, 0, Math.PI * 2);
    context.fill();
  } else if (mode === "voxel") {
    const half = size * 0.38;
    context.beginPath();
    context.moveTo(centerX, centerY - half);
    context.lineTo(centerX + half, centerY - half * 0.42);
    context.lineTo(centerX, centerY + half * 0.18);
    context.lineTo(centerX - half, centerY - half * 0.42);
    context.closePath();
    context.fillStyle = brightColor;
    context.fill();
    context.globalAlpha *= 0.8;
    context.fillStyle = color;
    context.fillRect(centerX - half, centerY - half * 0.4, half, half * 0.95);
    context.globalAlpha *= 0.72;
    context.fillRect(centerX, centerY - half * 0.4, half, half * 0.95);
  } else if (mode === "hearts") {
    const radius = size * 0.24;
    context.beginPath();
    context.moveTo(centerX, centerY + size * 0.42);
    context.bezierCurveTo(centerX - size * 0.55, centerY + size * 0.08, centerX - radius, centerY - size * 0.43, centerX, centerY - size * 0.12);
    context.bezierCurveTo(centerX + radius, centerY - size * 0.43, centerX + size * 0.55, centerY + size * 0.08, centerX, centerY + size * 0.42);
    context.fill();
  } else if (mode === "stars") {
    drawStar(context, centerX, centerY, size * 0.48, size * 0.21);
    context.fill();
  } else {
    const chars = parameters.customChars || CHARACTER_SETS[parameters.charSet];
    let glyph = chars[Math.min(chars.length - 1, Math.floor(luma * chars.length))] || " ";
    if (mode === "braille") glyph = String.fromCharCode(0x2800 + Math.max(1, Math.round(luma * 255)));
    if (mode === "hexdump") glyph = Math.round(luma * 15).toString(16).toUpperCase();
    if (mode === "matrix") {
      const fall = (cell.row + Math.floor(time / 85) + cell.col * 3) % 18;
      glyph = String.fromCharCode(0x30a0 + ((cell.col * 11 + cell.row * 7 + fall) % 86));
      context.fillStyle = fall < 2 ? "#d8ffdf" : `rgb(28 ${Math.round(128 + luma * 127)} 62)`;
      context.globalAlpha *= fall < 8 ? 1 : 0.42;
    }
    context.font = `${Math.max(7, size * (0.72 + luma * 0.38))}px "Geist Mono", monospace`;
    context.textAlign = "center";
    context.textBaseline = "middle";
    context.fillText(glyph, centerX, centerY + cellSize * 0.05);
  }
  context.restore();
}

function copyCanvas(source: HTMLCanvasElement) {
  const copy = document.createElement("canvas");
  copy.width = source.width;
  copy.height = source.height;
  copy.getContext("2d")?.drawImage(source, 0, 0);
  return copy;
}

function applyConfiguredBlur(canvas: HTMLCanvasElement, parameters: MosaicParameters) {
  if (parameters.blurType === "off" || parameters.blurAmount <= 0) return;
  const original = copyCanvas(canvas);
  const context = canvas.getContext("2d");
  if (!context) return;
  const amount = Math.max(0.5, parameters.blurAmount / 8);
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.save();

  if (parameters.blurType === "motion") {
    const radians = parameters.blurAngle * Math.PI / 180;
    const directions = parameters.directionalBothSides ? [-1, 1] : [1];
    context.globalAlpha = 0.16;
    directions.forEach((direction) => {
      for (let step = 0; step < 6; step += 1) {
        const distance = amount * step * direction;
        context.drawImage(original, Math.cos(radians) * distance, Math.sin(radians) * distance);
      }
    });
  } else {
    context.filter = `blur(${amount}px)`;
    context.drawImage(original, 0, 0);
    context.filter = "none";
    if (parameters.blurType !== "gaussian") {
      const sharp = copyCanvas(original);
      context.globalCompositeOperation = "source-over";
      if (parameters.blurType === "lens") {
        const mask = context.createRadialGradient(
          canvas.width * parameters.blurCenterX / 100,
          canvas.height * parameters.blurCenterY / 100,
          0,
          canvas.width * parameters.blurCenterX / 100,
          canvas.height * parameters.blurCenterY / 100,
          Math.max(canvas.width, canvas.height) * parameters.lensFocus / 100,
        );
        mask.addColorStop(0, "rgba(255,255,255,1)");
        mask.addColorStop(1, "rgba(255,255,255,0)");
        context.globalAlpha = 0.92;
        context.drawImage(sharp, 0, 0);
        context.globalCompositeOperation = "destination-in";
        context.fillStyle = mask;
        context.fillRect(0, 0, canvas.width, canvas.height);
      } else {
        const axis = parameters.blurType === "tilt" ? parameters.tiltPosition : parameters.progressivePosition;
        const feather = parameters.blurType === "tilt" ? parameters.tiltFeather : 22;
        const gradient = context.createLinearGradient(0, 0, 0, canvas.height);
        const start = clamp((axis - feather) / 100);
        const end = clamp((axis + feather) / 100);
        const reverse = parameters.progressiveReverse;
        gradient.addColorStop(0, reverse ? "rgba(255,255,255,0)" : "rgba(255,255,255,1)");
        gradient.addColorStop(start, reverse ? "rgba(255,255,255,0)" : "rgba(255,255,255,1)");
        gradient.addColorStop(end, reverse ? "rgba(255,255,255,1)" : "rgba(255,255,255,0)");
        gradient.addColorStop(1, reverse ? "rgba(255,255,255,1)" : "rgba(255,255,255,0)");
        const reveal = copyCanvas(sharp);
        const revealContext = reveal.getContext("2d");
        if (revealContext) {
          revealContext.globalCompositeOperation = "destination-in";
          revealContext.fillStyle = gradient;
          revealContext.fillRect(0, 0, reveal.width, reveal.height);
          context.globalCompositeOperation = "source-over";
          context.drawImage(reveal, 0, 0);
        }
      }
    }
  }
  context.restore();
}

function applyPostEffects(
  canvas: HTMLCanvasElement,
  bloomBuffer: HTMLCanvasElement,
  parameters: MosaicParameters,
  time: number,
) {
  const context = canvas.getContext("2d");
  if (!context) return;
  const { width, height } = canvas;

  if (parameters.pfx.bloom.enabled) {
    if (bloomBuffer.width !== width) bloomBuffer.width = width;
    if (bloomBuffer.height !== height) bloomBuffer.height = height;
    const bloomContext = bloomBuffer.getContext("2d");
    bloomContext?.clearRect(0, 0, width, height);
    bloomContext?.drawImage(canvas, 0, 0);
    context.save();
    context.globalCompositeOperation = "lighter";
    context.globalAlpha = parameters.pfx.bloom.intensity / 100 * 0.32;
    context.filter = `blur(${3 + parameters.pfx.bloom.intensity * 0.11}px)`;
    context.drawImage(bloomBuffer, 0, 0);
    context.restore();
  }

  if (parameters.pfx.chromatic.enabled) {
    const source = copyCanvas(canvas);
    const shift = Math.max(1, parameters.pfx.chromatic.intensity / 6);
    context.save();
    context.globalCompositeOperation = "screen";
    context.globalAlpha = 0.16;
    context.fillStyle = "#ff224f";
    context.drawImage(source, -shift, 0);
    context.globalCompositeOperation = "source-atop";
    context.fillRect(0, 0, width, height);
    context.globalCompositeOperation = "screen";
    context.fillStyle = "#35aaff";
    context.drawImage(source, shift, 0);
    context.globalCompositeOperation = "source-atop";
    context.fillRect(0, 0, width, height);
    context.restore();
  }

  if (parameters.pfx.scanLines.enabled) {
    context.save();
    context.globalAlpha = parameters.pfx.scanLines.intensity / 100 * 0.24;
    context.fillStyle = "#000";
    for (let y = 1; y < height; y += 4) context.fillRect(0, y, width, 1);
    context.restore();
  }

  if (parameters.pfx.halftone.enabled) {
    context.save();
    context.globalCompositeOperation = "multiply";
    context.fillStyle = "#050505";
    context.globalAlpha = parameters.pfx.halftone.intensity / 100 * 0.28;
    const spacing = Math.max(5, 12 - parameters.pfx.halftone.intensity / 12);
    for (let y = 0; y < height; y += spacing) {
      for (let x = 0; x < width; x += spacing) {
        context.beginPath();
        context.arc(x, y, 1.1, 0, Math.PI * 2);
        context.fill();
      }
    }
    context.restore();
  }

  if (parameters.pfx.glitch.enabled) {
    const source = copyCanvas(canvas);
    const strength = parameters.pfx.glitch.intensity / 100;
    context.save();
    for (let slice = 0; slice < 7; slice += 1) {
      const y = Math.floor(hash2d(slice, Math.floor(time / 90)) * height);
      const sliceHeight = 2 + Math.floor(hash2d(slice, 3) * 12 * strength);
      const offset = (hash2d(slice, 7, Math.floor(time / 140)) - 0.5) * 42 * strength;
      context.drawImage(source, 0, y, width, sliceHeight, offset, y, width, sliceHeight);
    }
    context.restore();
  }

  if (parameters.pfx.pixelate.enabled) {
    const source = copyCanvas(canvas);
    const scale = Math.max(0.08, 1 - parameters.pfx.pixelate.intensity / 108);
    const smallWidth = Math.max(1, Math.round(width * scale));
    const smallHeight = Math.max(1, Math.round(height * scale));
    const small = document.createElement("canvas");
    small.width = smallWidth;
    small.height = smallHeight;
    small.getContext("2d")?.drawImage(source, 0, 0, smallWidth, smallHeight);
    context.save();
    context.imageSmoothingEnabled = false;
    context.clearRect(0, 0, width, height);
    context.drawImage(small, 0, 0, width, height);
    context.restore();
  }

  if (parameters.pfx.filmGrain.enabled) {
    context.save();
    context.globalAlpha = parameters.pfx.filmGrain.intensity / 100 * 0.12;
    for (let index = 0; index < width * height / 420; index += 1) {
      const x = hash2d(index, 1, Math.floor(time / 66)) * width;
      const y = hash2d(index, 2, Math.floor(time / 66)) * height;
      context.fillStyle = hash2d(index, 3) > 0.5 ? "#fff" : "#000";
      context.fillRect(x, y, 1, 1);
    }
    context.restore();
  }

  if (parameters.pfx.filmDust.enabled) {
    context.save();
    context.strokeStyle = "#fff";
    context.fillStyle = "#fff";
    context.globalAlpha = parameters.pfx.filmDust.intensity / 100 * 0.25;
    for (let index = 0; index < 18; index += 1) {
      const x = hash2d(index, 9, Math.floor(time / 500)) * width;
      const y = hash2d(index, 10, Math.floor(time / 500)) * height;
      const size = 0.5 + hash2d(index, 11) * 2.5;
      context.beginPath();
      context.arc(x, y, size, 0, Math.PI * 2);
      context.fill();
    }
    context.restore();
  }

  if (parameters.pfx.vignette.enabled) {
    const strength = parameters.pfx.vignette.intensity / 100;
    const vignette = context.createRadialGradient(
      width * 0.58,
      height * 0.48,
      Math.min(width, height) * 0.08,
      width * 0.58,
      height * 0.48,
      Math.max(width, height) * 0.68,
    );
    vignette.addColorStop(0, "rgba(0,0,0,0)");
    vignette.addColorStop(0.58, `rgba(0,0,0,${strength * 0.08})`);
    vignette.addColorStop(1, `rgba(0,0,0,${strength * 0.82})`);
    context.fillStyle = vignette;
    context.fillRect(0, 0, width, height);
  }
}

function drawLights(canvas: HTMLCanvasElement, parameters: MosaicParameters) {
  if (!parameters.lights.enabled) return;
  const context = canvas.getContext("2d");
  if (!context) return;
  context.save();
  context.globalCompositeOperation = "screen";
  parameters.lights.points.forEach((point) => {
    const x = point.x * canvas.width;
    const y = point.y * canvas.height;
    const radius = point.radius * Math.max(canvas.width, canvas.height);
    const glow = context.createRadialGradient(x, y, 0, x, y, radius);
    glow.addColorStop(0, `rgba(255,255,255,${clamp(point.intensity) * 0.72})`);
    glow.addColorStop(0.34, `rgba(120,196,255,${clamp(point.intensity) * 0.28})`);
    glow.addColorStop(1, "rgba(0,0,0,0)");
    context.fillStyle = glow;
    context.fillRect(x - radius, y - radius, radius * 2, radius * 2);
  });
  context.restore();
}

function revealPlainPhoto(
  canvas: HTMLCanvasElement,
  photo: HTMLCanvasElement,
  maskImage: HTMLImageElement | null,
  parameters: MosaicParameters,
) {
  if (!parameters.mask.enabled || !maskImage) return;
  const reveal = copyCanvas(photo);
  const revealContext = reveal.getContext("2d");
  const context = canvas.getContext("2d");
  if (!revealContext || !context) return;
  revealContext.globalCompositeOperation = parameters.mask.invert ? "destination-out" : "destination-in";
  revealContext.drawImage(maskImage, 0, 0, reveal.width, reveal.height);
  context.drawImage(reveal, 0, 0);
}

function renderBackground(
  target: HTMLCanvasElement,
  photo: HTMLCanvasElement,
  buffers: RenderBuffers,
  cells: SampledCell[],
  width: number,
  height: number,
  pixelRatio: number,
  time: number,
  maskImage: HTMLImageElement | null,
  parameters: MosaicParameters,
) {
  const frame = buffers.frame;
  const frameContext = frame.getContext("2d");
  if (!frameContext) return;
  frameContext.clearRect(0, 0, width, height);

  if (parameters.bgMode === "solid") {
    frameContext.fillStyle = "#07100a";
    frameContext.fillRect(0, 0, width, height);
  } else if (parameters.bgMode === "original" || parameters.bgMode === "blurred") {
    frameContext.save();
    frameContext.globalAlpha = parameters.bgOpacity / 100;
    if (parameters.bgMode === "blurred") frameContext.filter = `blur(${parameters.bgBlur}px)`;
    frameContext.drawImage(photo, 0, 0);
    frameContext.restore();
  }

  const primitives = buffers.primitives;
  const primitiveContext = primitives.getContext("2d");
  if (!primitiveContext) return;
  primitiveContext.clearRect(0, 0, width, height);
  primitiveContext.globalCompositeOperation = parameters.styleBlend;
  cells.forEach((cell) => drawPrimitive(primitiveContext, cell, parameters, time, width, height));

  const adjusted = buffers.adjusted;
  const adjustedContext = adjusted.getContext("2d");
  if (!adjustedContext) return;
  adjustedContext.clearRect(0, 0, width, height);
  adjustedContext.filter = [
    `brightness(${100 + parameters.brightness}%)`,
    `contrast(${parameters.contrast}%)`,
    `saturate(${parameters.saturation}%)`,
    `grayscale(${parameters.grayscale}%)`,
  ].join(" ");
  adjustedContext.drawImage(primitives, 0, 0);
  adjustedContext.filter = "none";

  if (parameters.tintOpacity > 0) {
    adjustedContext.save();
    adjustedContext.globalCompositeOperation = parameters.overlayBlend;
    adjustedContext.globalAlpha = parameters.tintOpacity / 100;
    adjustedContext.fillStyle = parameters.tint;
    adjustedContext.fillRect(0, 0, width, height);
    adjustedContext.restore();
  }

  applyConfiguredBlur(adjusted, parameters);
  applyPostEffects(adjusted, buffers.bloom, parameters, time);
  drawLights(adjusted, parameters);
  frameContext.drawImage(adjusted, 0, 0);
  revealPlainPhoto(frame, photo, maskImage, parameters);

  const targetContext = target.getContext("2d");
  if (!targetContext) return;
  targetContext.setTransform(1, 0, 0, 1, 0, 0);
  targetContext.clearRect(0, 0, target.width, target.height);
  targetContext.imageSmoothingEnabled = true;
  targetContext.imageSmoothingQuality = "high";
  targetContext.drawImage(frame, 0, 0, width * pixelRatio, height * pixelRatio);
}

export function MosaicLeafBackground() {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const host = hostRef.current;
    const canvas = canvasRef.current;
    if (!host || !canvas) return undefined;

    const parameters = MOSAIC_LEAF_PARAMETERS;
    const photoImage = new Image();
    photoImage.decoding = "async";
    photoImage.src = mosaicLeafSource;
    const maskImage = parameters.mask.dataUrl ? new Image() : null;
    if (maskImage && parameters.mask.dataUrl) maskImage.src = parameters.mask.dataUrl;
    const photo = document.createElement("canvas");
    const buffers: RenderBuffers = {
      frame: document.createElement("canvas"),
      primitives: document.createElement("canvas"),
      adjusted: document.createElement("canvas"),
      bloom: document.createElement("canvas"),
    };
    let cells: SampledCell[] = [];
    let width = 0;
    let height = 0;
    let pixelRatio = 1;
    let animationFrame = 0;
    let resizeFrame = 0;
    let lastAnimationAt = -Infinity;
    let ready = false;
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

    const rebuild = () => {
      if (!ready) return;
      const bounds = host.getBoundingClientRect();
      const nextWidth = Math.max(1, Math.round(bounds.width));
      const nextHeight = Math.max(1, Math.round(bounds.height));
      if (nextWidth === width && nextHeight === height) return;
      width = nextWidth;
      height = nextHeight;
      pixelRatio = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.round(width * pixelRatio);
      canvas.height = Math.round(height * pixelRatio);
      photo.width = width;
      photo.height = height;
      Object.values(buffers).forEach((buffer) => {
        buffer.width = width;
        buffer.height = height;
      });
      const photoContext = photo.getContext("2d", { willReadFrequently: true });
      if (!photoContext) return;
      photoContext.clearRect(0, 0, width, height);
      drawImageCover(
        photoContext,
        photoImage,
        photoImage.naturalWidth,
        photoImage.naturalHeight,
        width,
        height,
        width * 0.21,
      );
      cells = sampleCells(photo, width, height, parameters.cellSize);
      renderBackground(canvas, photo, buffers, cells, width, height, pixelRatio, 0, maskImage, parameters);
    };

    const animate = (time: number) => {
      if (time - lastAnimationAt >= 1000 / 30) {
        renderBackground(canvas, photo, buffers, cells, width, height, pixelRatio, time, maskImage, parameters);
        lastAnimationAt = time;
      }
      const shouldAnimate = !reducedMotion.matches
        && ((parameters.animated && parameters.animSpeed.enabled) || parameters.renderMode === "matrix");
      if (shouldAnimate) animationFrame = requestAnimationFrame(animate);
    };

    const start = () => {
      cancelAnimationFrame(animationFrame);
      rebuild();
      if (width > 0 && height > 0) animationFrame = requestAnimationFrame(animate);
    };

    const loadPhoto = () => {
      ready = true;
      start();
    };
    photoImage.onload = loadPhoto;
    if (photoImage.complete && photoImage.naturalWidth > 0) loadPhoto();

    const resizeObserver = new ResizeObserver(() => {
      cancelAnimationFrame(resizeFrame);
      resizeFrame = requestAnimationFrame(() => {
        const oldWidth = width;
        const oldHeight = height;
        rebuild();
        if (oldWidth !== width || oldHeight !== height) start();
      });
    });
    resizeObserver.observe(host);
    reducedMotion.addEventListener("change", start);

    return () => {
      resizeObserver.disconnect();
      reducedMotion.removeEventListener("change", start);
      cancelAnimationFrame(animationFrame);
      cancelAnimationFrame(resizeFrame);
    };
  }, []);

  return (
    <div className="mosaic-leaf-background" ref={hostRef} aria-hidden="true">
      <canvas ref={canvasRef} />
    </div>
  );
}
