const canvas = document.getElementById('diagramCanvas');
const ctx = canvas ? canvas.getContext('2d') : null;

let currentTool = 'select';
let shapes = [];
let selectedShape = null;
let isDragging = false;
let isDrawing = false;
let drawingConnection = false;
let startX, startY;
let dragOffsetX, dragOffsetY;
let nextShapeId = 1;
let hoveredConnectionPoint = null;
let startConnectionPoint = null;

// ── View transform (zoom & pan) ───────────────────────────────
let viewScale   = 1;
let viewOffsetX = 0;
let viewOffsetY = 0;
let isPanning   = false;
let hasPanned   = false;   // true once pointer has moved past the dead-zone threshold
let panStartX   = 0;
let panStartY   = 0;

// ── Multi-select state ────────────────────────────────────────
let selectedShapes   = [];     // shapes in the current group selection
let lassoRect        = null;   // {x,y,w,h} world-coords while drawing lasso
let isMultiDragging  = false;
let multiDragOrigins = [];     // [{shape, ox, oy}] original positions when drag started
let multiDragStartX  = 0;
let multiDragStartY  = 0;

// ── Resize state ──────────────────────────────────────────────
let isResizing    = false;
let resizeHandle  = null;   // 'nw'|'n'|'ne'|'e'|'se'|'s'|'sw'|'w'
let resizeOrigX   = 0, resizeOrigY  = 0;
let resizeOrigW   = 0, resizeOrigH  = 0;
let resizeStartX  = 0, resizeStartY = 0;

const RESIZE_CURSORS = {
  nw: 'nw-resize', n: 'n-resize',  ne: 'ne-resize',
  e:  'e-resize',  se: 'se-resize', s: 's-resize',
  sw: 'sw-resize', w: 'w-resize'
};

function getResizeHandles(shape) {
  const { x, y, width: w, height: h } = shape;
  return {
    nw: { x,           y           },
    n:  { x: x + w/2,  y           },
    ne: { x: x + w,    y           },
    e:  { x: x + w,    y: y + h/2  },
    se: { x: x + w,    y: y + h    },
    s:  { x: x + w/2,  y: y + h    },
    sw: { x,           y: y + h    },
    w:  { x,           y: y + h/2  },
  };
}

function findResizeHandle(shape, x, y) {
  if (!isNodeShape(shape.type)) return null;
  const threshold = 7 / viewScale;
  for (const [name, pt] of Object.entries(getResizeHandles(shape))) {
    if (Math.hypot(x - pt.x, y - pt.y) < threshold) return name;
  }
  return null;
}

function screenToCanvas(sx, sy) {
  return { x: (sx - viewOffsetX) / viewScale, y: (sy - viewOffsetY) / viewScale };
}

function resetView() {
  viewScale = 1; viewOffsetX = 0; viewOffsetY = 0;
  redraw();
}

// Shape role groups
const NODE_SHAPES      = ['class', 'interface'];
const CONNECTOR_SHAPES = ['extends', 'implements', 'calls'];

function isNodeShape(type)      { return NODE_SHAPES.includes(type); }
function isConnectorShape(type) { return CONNECTOR_SHAPES.includes(type); }

// ── Shape class ───────────────────────────────────────────────
class Shape {
  constructor(type, x, y, width, height, text = '') {
    this.id    = nextShapeId++;
    this.type  = type;
    this.x     = x;
    this.y     = y;
    this.width  = width;
    this.height = height;
    this.text   = text;

    // Visual style
    this.strokeColor = '#333333';
    this.lineWidth   = 2;

    // Class/Interface members (stored directly on the node — no separate shapes needed)
    // method: { visibility, isStatic, isAbstract, name, params, returnType }
    this.methods = [];
    // field:  { visibility, isStatic, isFinal, fieldType, name }
    this.fields  = [];

    // Abstract flag (class nodes only)
    this.isAbstract = false;

    // Arrow label (for 'calls' connectors — e.g. "main -> add")
    this.label = '';

    // Connector anchors
    this.startNode  = null;
    this.startPoint = null;   // 'north' | 'south' | 'east' | 'west'
    this.endNode    = null;
    this.endPoint   = null;

    // Curved connector support
    this.curved      = true;   // curved by default — matches Graphviz edge routing
    this.curveOffset = 0;      // 0 = auto (20% of connector length); non-zero = manual override
  }

  // ── Border point computation ──────────────────────────────
  // Returns the world-coordinate point on this node's border at the given angle
  // from the node center. Works for both ellipse (class) and diamond (interface).
  _borderPoint(angle) {
    const cx = this.x + this.width  / 2;
    const cy = this.y + this.height / 2;
    const cosA = Math.cos(angle);
    const sinA = Math.sin(angle);
    if (this.type === 'class') {
      const rx = this.width  / 2;
      const ry = this.height / 2;
      return { x: cx + rx * cosA, y: cy + ry * sinA };
    }
    if (this.type === 'interface') {
      // Diamond |x/rx| + |y/ry| = 1 — solve for the scale factor t
      const rx = this.width  / 2;
      const ry = this.height / 2;
      const t  = 1 / (Math.abs(cosA) / rx + Math.abs(sinA) / ry);
      return { x: cx + t * cosA, y: cy + t * sinA };
    }
    return { x: cx, y: cy };
  }

  // ── Connection points ─────────────────────────────────────
  // Legacy: return the four cardinal points for backward-compat with old string-based anchors
  getConnectionPoints() {
    if (!isNodeShape(this.type)) return {};
    const cx = this.x + this.width  / 2;
    const cy = this.y + this.height / 2;
    return {
      north: { x: cx, y: this.y },
      south: { x: cx, y: this.y + this.height },
      east:  { x: this.x + this.width, y: cy },
      west:  { x: this.x, y: cy }
    };
  }

  // Accepts either a legacy string name ('north' etc.) or an angle object {kind:'angle', value:θ}
  getConnectionCoordinates(point) {
    if (point && typeof point === 'object' && point.kind === 'angle') {
      return this._borderPoint(point.value);
    }
    const pts = this.getConnectionPoints();
    return pts[point] || { x: this.x + this.width / 2, y: this.y + this.height / 2 };
  }

  // Returns {kind:'angle', value:θ} when the mouse is within THRESHOLD px of the border,
  // null otherwise. The snap point is the border point in the direction of the mouse from center.
  findConnectionPoint(x, y) {
    if (!isNodeShape(this.type)) return null;
    const cx    = this.x + this.width  / 2;
    const cy    = this.y + this.height / 2;
    const angle = Math.atan2(y - cy, x - cx);
    const bp    = this._borderPoint(angle);
    const THRESHOLD = 20;
    if (Math.hypot(x - bp.x, y - bp.y) < THRESHOLD) {
      return { kind: 'angle', value: angle };
    }
    return null;
  }

  // Draw a dashed glow ring around the whole border plus a snap dot at the hovered point
  drawConnectionPoints(highlight = false) {
    if (!isNodeShape(this.type)) return;
    const isHovered = hoveredConnectionPoint?.shape === this;
    const cx = this.x + this.width  / 2;
    const cy = this.y + this.height / 2;

    // Dashed border ring
    ctx.save();
    const gap = 6 / viewScale;
    ctx.setLineDash([5 / viewScale, 3 / viewScale]);
    ctx.strokeStyle = isHovered ? '#0d6efd' : (highlight ? '#6c757d' : '#adb5bd');
    ctx.lineWidth   = (isHovered ? 2 : 1.5) / viewScale;

    if (this.type === 'class') {
      ctx.beginPath();
      ctx.ellipse(cx, cy, this.width / 2 + gap, this.height / 2 + gap, 0, 0, 2 * Math.PI);
      ctx.stroke();
    } else if (this.type === 'interface') {
      ctx.beginPath();
      ctx.moveTo(cx,                          this.y - gap);
      ctx.lineTo(this.x + this.width  + gap,  cy);
      ctx.lineTo(cx,                          this.y + this.height + gap);
      ctx.lineTo(this.x - gap,                cy);
      ctx.closePath();
      ctx.stroke();
    }
    ctx.restore();

    // Snap dot at the currently hovered border point
    if (isHovered && hoveredConnectionPoint.point?.kind === 'angle') {
      const pt = this._borderPoint(hoveredConnectionPoint.point.value);
      ctx.beginPath();
      ctx.arc(pt.x, pt.y, 5 / viewScale, 0, 2 * Math.PI);
      ctx.fillStyle   = '#0d6efd';
      ctx.fill();
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth   = 1.5 / viewScale;
      ctx.stroke();
    }
  }

  drawResizeHandles() {
    const r = 5 / viewScale;  // constant screen-space size
    const lw = 1 / viewScale;
    Object.values(getResizeHandles(this)).forEach(pt => {
      ctx.beginPath();
      ctx.rect(pt.x - r, pt.y - r, r * 2, r * 2);
      ctx.fillStyle   = '#ffffff';
      ctx.fill();
      ctx.strokeStyle = '#0d6efd';
      ctx.lineWidth   = lw;
      ctx.stroke();
    });
  }

  // ── Connector helpers ─────────────────────────────────────
  getConnectorCoords() {
    let sX, sY, eX, eY;
    if (this.startNode && this.startPoint) {
      const c = this.startNode.getConnectionCoordinates(this.startPoint);
      sX = c.x; sY = c.y;
    } else { sX = this.x; sY = this.y; }
    if (this.endNode && this.endPoint) {
      const c = this.endNode.getConnectionCoordinates(this.endPoint);
      eX = c.x; eY = c.y;
    } else { eX = this.x + this.width; eY = this.y + this.height; }
    return { startX: sX, startY: sY, endX: eX, endY: eY };
  }

  _hollowArrowhead(endX, endY, angle, len) {
    ctx.beginPath();
    ctx.moveTo(endX, endY);
    ctx.lineTo(endX - len * Math.cos(angle - Math.PI / 6), endY - len * Math.sin(angle - Math.PI / 6));
    ctx.lineTo(endX - len * Math.cos(angle + Math.PI / 6), endY - len * Math.sin(angle + Math.PI / 6));
    ctx.closePath();
    ctx.fillStyle   = 'white';
    ctx.fill();
    ctx.strokeStyle = this.strokeColor;
    ctx.lineWidth   = 2;
    ctx.stroke();
  }

  _filledArrowhead(endX, endY, angle, len, color) {
    ctx.beginPath();
    ctx.moveTo(endX, endY);
    ctx.lineTo(endX - len * Math.cos(angle - Math.PI / 6), endY - len * Math.sin(angle - Math.PI / 6));
    ctx.lineTo(endX - len * Math.cos(angle + Math.PI / 6), endY - len * Math.sin(angle + Math.PI / 6));
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
  }

  // Quadratic bezier control point (perpendicular offset from midpoint).
  // When curveOffset===0 (auto), parallel connectors between the same node pair
  // are spread out symmetrically so they never overlap.
  _curveControlPoint(sx, sy, ex, ey) {
    const mx  = (sx + ex) / 2;
    const my  = (sy + ey) / 2;
    const dx  = ex - sx, dy = ey - sy;
    const len = Math.hypot(dx, dy) || 1;
    const nx  = -dy / len, ny = dx / len;  // perpendicular unit vector (90° CCW)

    let off;
    if (this.curveOffset !== 0) {
      // Manual override from the property panel
      off = this.curveOffset;
    } else if (this.startNode && this.endNode) {
      // Find all connectors that share exactly the same start→end node pair
      const parallel = shapes.filter(s =>
        isConnectorShape(s.type) &&
        s.startNode === this.startNode &&
        s.endNode   === this.endNode
      );
      const total = parallel.length;
      const idx   = parallel.indexOf(this);

      if (total <= 1) {
        // Single connector — gentle fixed curve so it doesn't look like a straight line
        const magnitude = Math.min(80, Math.max(25, len * 0.15));
        off = magnitude;
      } else {
        // Multiple connectors — spread symmetrically around the straight line.
        // step scales with distance so arrows don't crowd on short edges.
        const step = Math.min(65, Math.max(30, len * 0.13));
        const center = (total - 1) / 2;
        off = (idx - center) * step;
      }
    } else {
      // Unattached preview line — small fixed offset
      const magnitude = Math.min(80, Math.max(25, len * 0.15));
      off = magnitude;
    }
    return { cx: mx + nx * off, cy: my + ny * off };
  }

  _midLabel(sx, sy, ex, ey, label) {
    let lx, ly;
    if (this.curved) {
      const { cx, cy } = this._curveControlPoint(sx, sy, ex, ey);
      lx = 0.25 * sx + 0.5 * cx + 0.25 * ex;
      ly = 0.25 * sy + 0.5 * cy + 0.25 * ey;
    } else {
      lx = (sx + ex) / 2;
      ly = (sy + ey) / 2;
    }

    // Offset the label perpendicularly to the arrow so it sits beside, not on, the line
    const dx = ex - sx, dy = ey - sy;
    const len = Math.hypot(dx, dy) || 1;
    const nx = -dy / len, ny = dx / len;   // 90° CCW perpendicular unit vector
    const OFFSET = 14;
    lx += nx * OFFSET;
    ly += ny * OFFSET;

    ctx.font         = '11px Arial';
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'middle';

    // White pill background so the label never overlaps the line visually
    const tw = ctx.measureText(label).width;
    const th = 12;
    const pad = 3;
    ctx.fillStyle = 'rgba(255,255,255,0.9)';
    ctx.beginPath();
    ctx.roundRect(lx - tw / 2 - pad, ly - th / 2 - pad, tw + pad * 2, th + pad * 2, 3);
    ctx.fill();

    ctx.fillStyle = '#444';
    ctx.fillText(label, lx, ly);
  }

  // ── Main draw dispatcher ──────────────────────────────────
  draw() {
    const inGroup = selectedShapes.includes(this);
    ctx.lineWidth   = (selectedShape === this || inGroup) ? this.lineWidth + 1 : this.lineWidth;
    ctx.strokeStyle = this.strokeColor;
    if      (selectedShape === this) { ctx.shadowColor = '#0d6efd'; ctx.shadowBlur = 10; }
    else if (inGroup)                { ctx.shadowColor = '#fd7e14'; ctx.shadowBlur = 8;  }

    switch (this.type) {
      case 'class':      this._drawClass();           break;
      case 'interface':  this._drawInterface();        break;
      case 'extends':    this._drawExtendsArrow();     break;
      case 'implements': this._drawImplementsArrow();  break;
      case 'calls':      this._drawCallsArrow();       break;
    }

    ctx.shadowColor = 'transparent';
    ctx.shadowBlur  = 0;

    if (isNodeShape(this.type)) {
      if (selectedShape === this && currentTool === 'select') {
        this.drawResizeHandles();
      } else if (isConnectorShape(currentTool)) {
        this.drawConnectionPoints(selectedShape === this);
      }
    }
  }

  // ── Node draw methods ─────────────────────────────────────

  // Rounded-rectangle path helper (does not stroke/fill — caller does that)
  _roundRect(x, y, w, h, r) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + w - r, y);
    ctx.quadraticCurveTo(x + w, y, x + w, y + r);
    ctx.lineTo(x + w, y + h - r);
    ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
    ctx.lineTo(x + r, y + h);
    ctx.quadraticCurveTo(x, y + h, x, y + h - r);
    ctx.lineTo(x, y + r);
    ctx.quadraticCurveTo(x, y, x + r, y);
    ctx.closePath();
  }

  // ── Layout helpers ────────────────────────────────────────
  // Build method label string from a method data object
  static _methodLabel(m) {
    const parts = [];
    if (m.visibility && m.visibility !== 'package') parts.push(m.visibility);
    if (m.isStatic)   parts.push('static');
    if (m.isAbstract) parts.push('abstract');
    const prefix = parts.join(' ');
    return `${prefix}${prefix ? ' ' : ''}${m.name || 'method'}(${m.params || ''}): ${m.returnType || 'void'}`;
  }

  // Build field label for DOT output — omits visibility (matches backend format_field_for_diagram)
  static _fieldLabel(f) {
    const parts = [];
    if (f.isStatic)  parts.push('static');
    if (f.isFinal)   parts.push('final');
    if (f.fieldType) parts.push(f.fieldType);
    parts.push(f.name || 'field');
    return parts.join(' ');
  }

  // Build field label for canvas display — includes visibility so the user can see it
  static _fieldCanvasLabel(f) {
    const parts = [];
    if (f.visibility && f.visibility !== 'package') parts.push(f.visibility);
    if (f.isStatic)  parts.push('static');
    if (f.isFinal)   parts.push('final');
    if (f.fieldType) parts.push(f.fieldType);
    parts.push(f.name || 'field');
    return parts.join(' ');
  }

  // Draw the content (name, methods, fields) inside a node's bounding box.
  // `clipPath` is a function that sets up the clip region before drawing.
  _drawNodeContent(cx, clipPath) {
    const PAD    = 14;
    const NAME_H = 22;
    const ROW_H  = 17;
    const FIELD_H = 19;

    const name = this.text || (this.type === 'interface' ? 'InterfaceName' : 'ClassName');
    const nameLabel = this.type === 'interface' ? `${name} (interface)`
                    : this.isAbstract           ? `${name} (abstract)`
                    : name;

    const totalH = NAME_H
                 + this.methods.length * ROW_H
                 + (this.fields.length > 0 ? 4 + this.fields.length * FIELD_H : 0);

    let yPos = this.y + this.height / 2 - totalH / 2;

    ctx.save();
    clipPath();
    ctx.clip();

    // Class name — bold
    ctx.font         = 'bold 13px Arial';
    ctx.fillStyle    = '#000';
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(nameLabel, cx, yPos + NAME_H / 2, this.width - PAD * 2);
    yPos += NAME_H;

    // Methods — centered with underline
    ctx.font      = '11px Arial';
    ctx.textAlign = 'center';
    const textW = this.width - PAD * 2;
    this.methods.forEach(m => {
      const lbl = Shape._methodLabel(m);
      ctx.fillStyle = '#000';
      ctx.fillText(lbl, cx, yPos + ROW_H / 2, textW);
      // Underline — centered under the text
      const tw = Math.min(ctx.measureText(lbl).width, textW);
      ctx.beginPath();
      ctx.moveTo(cx - tw / 2, yPos + ROW_H / 2 + 7);
      ctx.lineTo(cx + tw / 2, yPos + ROW_H / 2 + 7);
      ctx.strokeStyle = '#000';
      ctx.lineWidth   = 0.8;
      ctx.setLineDash([]);
      ctx.stroke();
      yPos += ROW_H;
    });

    // Fields — yellow boxes, inset to stay within ellipse boundary
    if (this.fields.length > 0) {
      yPos += 4;
      const cy_ell = this.y + this.height / 2;
      const rx = this.width / 2;
      const ry = this.height / 2;
      this.fields.forEach(f => {
        const lbl = Shape._fieldCanvasLabel(f);
        const fieldCenterY = yPos + (FIELD_H - 2) / 2;
        const dy = fieldCenterY - cy_ell;
        const t = Math.abs(dy) < ry ? Math.sqrt(Math.max(0, 1 - (dy * dy) / (ry * ry))) : 0;
        const availW = 2 * rx * t;
        // Use 75% of available ellipse width at this y, capped at (width - PAD*4)
        const boxW = Math.max(60, Math.min(this.width - PAD * 4, availW * 0.75));
        const boxX = cx - boxW / 2;
        ctx.fillStyle = '#ffffe0';
        ctx.fillRect(boxX, yPos, boxW, FIELD_H - 2);
        ctx.strokeStyle = '#888';
        ctx.lineWidth   = 0.8;
        ctx.setLineDash([]);
        ctx.strokeRect(boxX, yPos, boxW, FIELD_H - 2);
        ctx.fillStyle    = '#000';
        ctx.textAlign    = 'center';
        ctx.font         = '11px Arial';
        ctx.fillText(lbl, cx, yPos + (FIELD_H - 2) / 2, boxW - 8);
        yPos += FIELD_H;
      });
    }

    ctx.restore();
  }

  // Grow the node to fit its current content (never shrinks below initial minimums)
  autoResizeForContent() {
    if (!isNodeShape(this.type) || !ctx) return;
    const PAD = 14, NAME_H = 22, ROW_H = 17, FIELD_H = 19;
    const MIN_W = 200, MIN_H = 160;

    const contentH = NAME_H
      + this.methods.length * ROW_H
      + (this.fields.length > 0 ? 4 + this.fields.length * FIELD_H : 0);
    const neededH = contentH + PAD * 4;

    // Measure widest text to determine minimum width
    const savedFont = ctx.font;
    let maxTextW = 0;
    const nameLabel = this.type === 'interface' ? `${this.text || 'InterfaceName'} (interface)`
                    : this.isAbstract           ? `${this.text || 'ClassName'} (abstract)`
                    : (this.text || 'ClassName');
    ctx.font = 'bold 13px Arial';
    maxTextW = Math.max(maxTextW, ctx.measureText(nameLabel).width);
    ctx.font = '11px Arial';
    this.methods.forEach(m => {
      maxTextW = Math.max(maxTextW, ctx.measureText(Shape._methodLabel(m)).width);
    });
    this.fields.forEach(f => {
      maxTextW = Math.max(maxTextW, ctx.measureText(Shape._fieldCanvasLabel(f)).width);
    });
    ctx.font = savedFont;

    // For an ellipse the visible text area is narrower than the full width — add generous padding
    const neededW = maxTextW + PAD * 6;

    this.height = Math.max(this.height, MIN_H, neededH);
    this.width  = Math.max(this.width,  MIN_W, neededW);
  }

  _drawClass() {
    const lw = selectedShape === this ? 3 : 2;
    const cx = this.x + this.width  / 2;
    const cy = this.y + this.height / 2;
    const rx = this.width  / 2;
    const ry = this.height / 2;

    ctx.beginPath();
    ctx.ellipse(cx, cy, rx, ry, 0, 0, 2 * Math.PI);
    ctx.fillStyle   = 'white';
    ctx.fill();
    ctx.strokeStyle = selectedShape === this ? '#0d6efd' : this.strokeColor;
    ctx.lineWidth   = lw;
    ctx.setLineDash([]);
    ctx.stroke();

    if (this.isAbstract) {
      const sh = 5;
      ctx.beginPath();
      ctx.ellipse(cx, cy, Math.max(4, rx - sh), Math.max(4, ry - sh), 0, 0, 2 * Math.PI);
      ctx.stroke();
    }

    this._drawNodeContent(cx, () => {
      ctx.beginPath();
      ctx.ellipse(cx, cy, Math.max(1, rx - lw), Math.max(1, ry - lw), 0, 0, 2 * Math.PI);
    });
  }

  _drawInterface() {
    const lw = selectedShape === this ? 3 : 2;
    const cx = this.x + this.width  / 2;
    const cy = this.y + this.height / 2;

    ctx.beginPath();
    ctx.moveTo(cx,                    this.y);
    ctx.lineTo(this.x + this.width,   cy);
    ctx.lineTo(cx,                    this.y + this.height);
    ctx.lineTo(this.x,                cy);
    ctx.closePath();
    ctx.fillStyle   = 'white';
    ctx.fill();
    ctx.strokeStyle = selectedShape === this ? '#0d6efd' : this.strokeColor;
    ctx.lineWidth   = lw;
    ctx.setLineDash([]);
    ctx.stroke();

    // Clip to the diamond for content
    this._drawNodeContent(cx, () => {
      ctx.beginPath();
      ctx.moveTo(cx,                        this.y + lw);
      ctx.lineTo(this.x + this.width - lw,  cy);
      ctx.lineTo(cx,                        this.y + this.height - lw);
      ctx.lineTo(this.x + lw,               cy);
      ctx.closePath();
    });
  }

  // ── Connector draw methods ────────────────────────────────
  // Draw a path (moveTo already done) — straight or curved quadratic bezier
  _pathLine(sx, sy, ex, ey) {
    if (this.curved) {
      const { cx, cy } = this._curveControlPoint(sx, sy, ex, ey);
      ctx.quadraticCurveTo(cx, cy, ex, ey);
    } else {
      ctx.lineTo(ex, ey);
    }
  }

  // Endpoint angle: tangent at t=1 of the bezier (control→end), or straight angle
  _endAngle(sx, sy, ex, ey) {
    if (this.curved) {
      const { cx, cy } = this._curveControlPoint(sx, sy, ex, ey);
      return Math.atan2(ey - cy, ex - cx);
    }
    return Math.atan2(ey - sy, ex - sx);
  }

  _drawExtendsArrow() {
    const { startX: sx, startY: sy, endX: ex, endY: ey } = this.getConnectorCoords();
    const LEN   = 18;
    const angle = this._endAngle(sx, sy, ex, ey);
    const tipX  = ex - LEN * Math.cos(angle);
    const tipY  = ey - LEN * Math.sin(angle);
    ctx.setLineDash([]);
    ctx.strokeStyle = this.strokeColor;
    ctx.lineWidth   = selectedShape === this ? 3 : 2;
    ctx.beginPath();
    ctx.moveTo(sx, sy);
    if (this.curved) {
      const { cx, cy } = this._curveControlPoint(sx, sy, ex, ey);
      // Curve stops just before the arrowhead tip
      const tRatio = 1 - LEN / Math.hypot(ex - sx, ey - sy);
      const stX = (1 - tRatio) * (1 - tRatio) * sx + 2 * (1 - tRatio) * tRatio * cx + tRatio * tRatio * ex;
      const stY = (1 - tRatio) * (1 - tRatio) * sy + 2 * (1 - tRatio) * tRatio * cy + tRatio * tRatio * ey;
      ctx.quadraticCurveTo(cx, cy, stX, stY);
    } else {
      ctx.lineTo(tipX, tipY);
    }
    ctx.stroke();
    this._hollowArrowhead(ex, ey, angle, LEN);
    this._midLabel(sx, sy, ex, ey, 'extends');
  }

  _drawImplementsArrow() {
    const { startX: sx, startY: sy, endX: ex, endY: ey } = this.getConnectorCoords();
    const LEN   = 18;
    const angle = this._endAngle(sx, sy, ex, ey);
    const tipX  = ex - LEN * Math.cos(angle);
    const tipY  = ey - LEN * Math.sin(angle);
    ctx.setLineDash([8, 4]);
    ctx.strokeStyle = this.strokeColor;
    ctx.lineWidth   = selectedShape === this ? 3 : 2;
    ctx.beginPath();
    ctx.moveTo(sx, sy);
    if (this.curved) {
      const { cx, cy } = this._curveControlPoint(sx, sy, ex, ey);
      const tRatio = 1 - LEN / Math.hypot(ex - sx, ey - sy);
      const stX = (1 - tRatio) * (1 - tRatio) * sx + 2 * (1 - tRatio) * tRatio * cx + tRatio * tRatio * ex;
      const stY = (1 - tRatio) * (1 - tRatio) * sy + 2 * (1 - tRatio) * tRatio * cy + tRatio * tRatio * ey;
      ctx.quadraticCurveTo(cx, cy, stX, stY);
    } else {
      ctx.lineTo(tipX, tipY);
    }
    ctx.stroke();
    ctx.setLineDash([]);
    this._hollowArrowhead(ex, ey, angle, LEN);
    this._midLabel(sx, sy, ex, ey, 'implements');
  }

  _drawCallsArrow() {
    const { startX: sx, startY: sy, endX: ex, endY: ey } = this.getConnectorCoords();
    const LEN   = 14;
    const angle = this._endAngle(sx, sy, ex, ey);
    ctx.setLineDash([]);
    ctx.strokeStyle = '#0000cd';
    ctx.lineWidth   = selectedShape === this ? 3 : 2;
    ctx.beginPath();
    ctx.moveTo(sx, sy);
    this._pathLine(sx, sy, ex, ey);
    ctx.stroke();
    this._filledArrowhead(ex, ey, angle, LEN, '#0000cd');
    if (this.label) this._midLabel(sx, sy, ex, ey, this.label);
  }

  // ── Hit testing ───────────────────────────────────────────
  contains(x, y) {
    if (isConnectorShape(this.type)) return this._containsLine(x, y);
    if (this.type === 'class') {
      const cx = this.x + this.width  / 2;
      const cy = this.y + this.height / 2;
      const rx = this.width  / 2;
      const ry = this.height / 2;
      const dx = (x - cx) / rx;
      const dy = (y - cy) / ry;
      return dx * dx + dy * dy <= 1;
    }
    if (this.type === 'interface') {
      const cx = this.x + this.width  / 2;
      const cy = this.y + this.height / 2;
      const dx = Math.abs(x - cx) / (this.width  / 2);
      const dy = Math.abs(y - cy) / (this.height / 2);
      return dx + dy <= 1;
    }
    return x >= this.x && x <= this.x + this.width &&
           y >= this.y && y <= this.y + this.height;
  }

  _containsLine(x, y) {
    const { startX: sx, startY: sy, endX: ex, endY: ey } = this.getConnectorCoords();
    if (!this.curved) return this._ptLineDist(x, y, sx, sy, ex, ey) < 8;
    // Sample points along the quadratic bezier for hit testing
    const { cx, cy } = this._curveControlPoint(sx, sy, ex, ey);
    for (let t = 0; t <= 1; t += 0.05) {
      const bx = (1 - t) * (1 - t) * sx + 2 * (1 - t) * t * cx + t * t * ex;
      const by = (1 - t) * (1 - t) * sy + 2 * (1 - t) * t * cy + t * t * ey;
      if (Math.hypot(x - bx, y - by) < 8) return true;
    }
    return false;
  }

  _ptLineDist(px, py, x1, y1, x2, y2) {
    const dx = x2 - x1, dy = y2 - y1;
    const len2 = dx * dx + dy * dy;
    if (len2 === 0) return Math.hypot(px - x1, py - y1);
    const t = Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / len2));
    return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy));
  }
}

// ── Tool buttons ──────────────────────────────────────────────
document.querySelectorAll('.tool-button').forEach(button => {
  button.addEventListener('click', function () {
    document.querySelectorAll('.tool-button').forEach(b => b.classList.remove('active'));
    this.classList.add('active');
    currentTool = this.dataset.tool;
    if (canvas && !isPanning) canvas.style.cursor = currentTool === 'select' ? 'default' : 'crosshair';
  });
});

// ── Canvas event handlers ─────────────────────────────────────
if (canvas) {
  // Resize canvas to fill its container
  const canvasContainer = canvas.parentElement;
  function fitCanvas() {
    const w = canvasContainer.clientWidth;
    const h = canvasContainer.clientHeight;
    if (w === 0 || h === 0) return; // page is hidden, skip
    canvas.width  = w;
    canvas.height = h;
    redraw();
  }
  // Expose so goTo('Creator') can trigger a re-fit after the page becomes visible
  window.fitCreatorCanvas = fitCanvas;
  fitCanvas();
  window.addEventListener('resize', fitCanvas);

  canvas.addEventListener('mousedown', handleMouseDown);
  canvas.addEventListener('mousemove', handleMouseMove);
  canvas.addEventListener('mouseup',   handleMouseUp);
  canvas.addEventListener('dblclick',  handleDoubleClick);

  // Prevent middle-mouse scroll / autoscroll popup
  canvas.addEventListener('mousedown', e => { if (e.button === 1) e.preventDefault(); });
  canvas.addEventListener('auxclick',  e => e.preventDefault());

  // Wheel zoom
  canvas.addEventListener('wheel', e => {
    e.preventDefault();
    const rect   = canvas.getBoundingClientRect();
    const mx     = e.clientX - rect.left;
    const my     = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    viewOffsetX  = mx + (viewOffsetX - mx) * factor;
    viewOffsetY  = my + (viewOffsetY - my) * factor;
    viewScale    = Math.max(0.1, Math.min(8, viewScale * factor));
    redraw();
  }, { passive: false });

  // Stop panning / resizing / lasso even if mouse is released outside the canvas
  window.addEventListener('mouseup', e => {
    if (isMultiDragging && e.button === 0) {
      isMultiDragging = false; multiDragOrigins = [];
      canvas.style.cursor = 'default';
      updateDOTPreview();
    }
    if (lassoRect && e.button === 0) { lassoRect = null; redraw(); }
    if (isResizing && e.button === 0) {
      isResizing = false; resizeHandle = null;
      canvas.style.cursor = 'default';
      updateDOTPreview();
    }
    if (isPanning && (e.button === 1 || e.button === 0)) {
      isPanning = false;
      canvas.style.cursor = currentTool === 'select' ? 'default' : 'crosshair';
    }
  });
}

function handleMouseDown(e) {
  const rect = canvas.getBoundingClientRect();
  const sx   = e.clientX - rect.left;
  const sy   = e.clientY - rect.top;

  // Middle-mouse → pan
  if (e.button === 1) {
    e.preventDefault();
    isPanning  = true;
    hasPanned  = false;
    panStartX  = e.clientX;
    panStartY  = e.clientY;
    canvas.style.cursor = 'grabbing';
    return;
  }

  if (e.button !== 0) return;

  const { x, y } = screenToCanvas(sx, sy);

  if (currentTool === 'select' || currentTool === 'lasso') {
    // Check resize handles on single-selected shape first (select tool only)
    if (currentTool === 'select' && selectedShape && isNodeShape(selectedShape.type)) {
      const handle = findResizeHandle(selectedShape, x, y);
      if (handle) {
        isResizing   = true;
        resizeHandle = handle;
        resizeOrigX  = selectedShape.x;
        resizeOrigY  = selectedShape.y;
        resizeOrigW  = selectedShape.width;
        resizeOrigH  = selectedShape.height;
        resizeStartX = x;
        resizeStartY = y;
        canvas.style.cursor = RESIZE_CURSORS[handle];
        return;
      }
    }

    // Check if clicking on a shape in the current group → start group drag
    if (selectedShapes.length > 0) {
      const hit = [...shapes].reverse().find(s => selectedShapes.includes(s) && s.contains(x, y));
      if (hit) {
        isMultiDragging  = true;
        multiDragStartX  = x;
        multiDragStartY  = y;
        multiDragOrigins = selectedShapes.map(s => ({ shape: s, ox: s.x, oy: s.y }));
        canvas.style.cursor = 'move';
        return;
      }
    }

    // In select mode: check shape hit for single selection
    if (currentTool === 'select') {
      for (let i = shapes.length - 1; i >= 0; i--) {
        if (shapes[i].contains(x, y)) {
          const clicked = shapes[i];

          if (e.ctrlKey) {
            // Ctrl+click: migrate single selectedShape into selectedShapes if needed
            if (selectedShape && !selectedShapes.includes(selectedShape)) {
              selectedShapes.push(selectedShape);
              selectedShape = null;
            }
            // Toggle clicked shape in the group
            const idx = selectedShapes.indexOf(clicked);
            if (idx >= 0) {
              selectedShapes.splice(idx, 1);
            } else {
              selectedShapes.push(clicked);
            }
            updatePropertyEditor();
            redraw();
            return;
          }

          selectedShapes = [];
          selectedShape  = clicked;
          isDragging     = true;
          dragOffsetX    = x - selectedShape.x;
          dragOffsetY    = y - selectedShape.y;
          updatePropertyEditor();
          redraw();
          return;
        }
      }
      // Clicked empty space — clear group (unless Ctrl held), start pan
      if (!e.ctrlKey) selectedShapes = [];
      isPanning  = true;
      hasPanned  = false;
      panStartX  = e.clientX;
      panStartY  = e.clientY;
      canvas.style.cursor = 'grabbing';

    } else {
      // Lasso tool — start drawing selection rect
      selectedShape  = null;
      selectedShapes = [];
      lassoRect = { x, y, w: 0, h: 0 };
      updatePropertyEditor();
    }

  } else if (isConnectorShape(currentTool)) {
    // Try to start from a connection point
    for (let i = shapes.length - 1; i >= 0; i--) {
      const shape = shapes[i];
      if (!isNodeShape(shape.type)) continue;
      const pt = shape.findConnectionPoint(x, y);
      if (pt) {
        drawingConnection = true;
        startConnectionPoint = { shape, point: pt };
        const coords = shape.getConnectionCoordinates(pt);
        startX = coords.x; startY = coords.y;
        return;
      }
    }

  } else if (isNodeShape(currentTool)) {
    isDrawing = true;
    startX = x; startY = y;
  }
}

function handleMouseMove(e) {
  // Handle pan
  if (isPanning) {
    const dx = e.clientX - panStartX;
    const dy = e.clientY - panStartY;
    if (!hasPanned && Math.hypot(dx, dy) > 3) hasPanned = true;
    if (hasPanned) {
      viewOffsetX += dx;
      viewOffsetY += dy;
      panStartX    = e.clientX;
      panStartY    = e.clientY;
      canvas.style.cursor = 'grabbing';
      redraw();
    }
    return;
  }

  const rect       = canvas.getBoundingClientRect();
  const sx         = e.clientX - rect.left;
  const sy         = e.clientY - rect.top;
  const { x, y }  = screenToCanvas(sx, sy);

  // Handle group drag
  if (isMultiDragging) {
    const dx = x - multiDragStartX;
    const dy = y - multiDragStartY;
    multiDragOrigins.forEach(({ shape, ox, oy }) => { shape.x = ox + dx; shape.y = oy + dy; });
    redraw();
    return;
  }

  // Update lasso rect
  if (lassoRect) {
    lassoRect.w = x - lassoRect.x;
    lassoRect.h = y - lassoRect.y;
    redraw();
    return;
  }

  // Handle active resize
  if (isResizing && selectedShape) {
    const MIN = 20;
    const dx = x - resizeStartX;
    const dy = y - resizeStartY;
    let nx = resizeOrigX, ny = resizeOrigY, nw = resizeOrigW, nh = resizeOrigH;
    if (resizeHandle.includes('e')) nw = Math.max(MIN, resizeOrigW + dx);
    if (resizeHandle.includes('s')) nh = Math.max(MIN, resizeOrigH + dy);
    if (resizeHandle.includes('w')) { nw = Math.max(MIN, resizeOrigW - dx); nx = resizeOrigX + resizeOrigW - nw; }
    if (resizeHandle.includes('n')) { nh = Math.max(MIN, resizeOrigH - dy); ny = resizeOrigY + resizeOrigH - nh; }
    selectedShape.x = nx; selectedShape.y = ny;
    selectedShape.width = nw; selectedShape.height = nh;
    redraw();
    return;
  }

  // Update hovered connection point
  hoveredConnectionPoint = null;
  if (isConnectorShape(currentTool)) {
    for (const shape of shapes) {
      if (!isNodeShape(shape.type)) continue;
      const pt = shape.findConnectionPoint(x, y);
      if (pt) {
        hoveredConnectionPoint = { shape, point: pt };
        canvas.style.cursor = 'crosshair';
        if (!isDragging && !isDrawing && !drawingConnection) redraw();
        break;
      }
    }
  }

  // Update cursor when hovering over resize handles of selected shape
  if (currentTool === 'select' && selectedShape && isNodeShape(selectedShape.type) && !isDragging) {
    const handle = findResizeHandle(selectedShape, x, y);
    if (handle) {
      canvas.style.cursor = RESIZE_CURSORS[handle];
    } else if (!hoveredConnectionPoint) {
      canvas.style.cursor = 'default';
    }
  }

  if (isDragging && selectedShape && isNodeShape(selectedShape.type)) {
    selectedShape.x = x - dragOffsetX;
    selectedShape.y = y - dragOffsetY;
    redraw();
    updateDOTPreview();
  } else if (drawingConnection || isDrawing) {
    redraw();
    // Draw the preview line in world-space (transform already applied)
    ctx.setTransform(viewScale, 0, 0, viewScale, viewOffsetX, viewOffsetY);
    ctx.strokeStyle = '#888';
    ctx.lineWidth   = 1;
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(startX, startY);
    ctx.lineTo(x, y);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
  }
}

function handleMouseUp(e) {
  if (isMultiDragging) {
    isMultiDragging  = false;
    multiDragOrigins = [];
    canvas.style.cursor = 'default';
    updateDOTPreview();
    return;
  }

  if (lassoRect) {
    // Normalise rect so w/h are always positive
    const rx = lassoRect.w >= 0 ? lassoRect.x : lassoRect.x + lassoRect.w;
    const ry = lassoRect.h >= 0 ? lassoRect.y : lassoRect.y + lassoRect.h;
    const rw = Math.abs(lassoRect.w);
    const rh = Math.abs(lassoRect.h);
    lassoRect = null;

    if (rw > 4 && rh > 4) {
      selectedShapes = shapes.filter(s => {
        if (!isNodeShape(s.type)) return false;
        // shape must be fully inside the lasso rect
        return s.x >= rx && s.y >= ry && s.x + s.width <= rx + rw && s.y + s.height <= ry + rh;
      });
      selectedShape = null;
      updatePropertyEditor();
    }
    redraw();
    return;
  }

  if (isResizing) {
    isResizing   = false;
    resizeHandle = null;
    canvas.style.cursor = 'default';
    updateDOTPreview();
    return;
  }

  if (isPanning) {
    isPanning = false;
    if (!hasPanned && !e.ctrlKey) {
      // Treat as a plain click on empty space → deselect
      selectedShape  = null;
      selectedShapes = [];
      updatePropertyEditor();
      redraw();
    }
    hasPanned = false;
    canvas.style.cursor = currentTool === 'select' ? 'default' : 'crosshair';
    return;
  }

  const rect      = canvas.getBoundingClientRect();
  const sx        = e.clientX - rect.left;
  const sy        = e.clientY - rect.top;
  const { x, y } = screenToCanvas(sx, sy);

  if (drawingConnection) {
    for (const shape of shapes) {
      if (!isNodeShape(shape.type)) continue;
      const pt = shape.findConnectionPoint(x, y);
      if (pt && shape !== startConnectionPoint.shape) {
        const conn      = new Shape(currentTool, 0, 0, 0, 0);
        conn.startNode  = startConnectionPoint.shape;
        conn.startPoint = startConnectionPoint.point;
        conn.endNode    = shape;
        conn.endPoint   = pt;
        shapes.push(conn);
        updateDOTPreview();
        break;
      }
    }
    drawingConnection = false;
    startConnectionPoint = null;
    redraw();

  } else if (isDrawing) {
    if (isNodeShape(currentTool)) {
      let dw = Math.abs(x - startX);
      let dh = Math.abs(y - startY);
      if (dw < 15) dw = 200;
      if (dh < 15) dh = 160;

      const shape = new Shape(
        currentTool,
        Math.min(startX, x),
        Math.min(startY, y),
        dw, dh
      );
      shapes.push(shape);
      selectedShape = shape;
      updatePropertyEditor();
      updateDOTPreview();
    }
    isDrawing = false;
    redraw();
  }

  isDragging = false;
}

function handleDoubleClick(e) {
  const rect      = canvas.getBoundingClientRect();
  const sx        = e.clientX - rect.left;
  const sy        = e.clientY - rect.top;
  const { x, y } = screenToCanvas(sx, sy);

  if (!selectedShape || !isNodeShape(selectedShape.type)) return;
  if (!selectedShape.contains(x, y)) return;

  const text = prompt('Class / Interface name:', selectedShape.text);
  if (text !== null) {
    selectedShape.text = text;
    updatePropertyEditor();
    updateDOTPreview();
    redraw();
  }
}

function redraw() {
  if (!ctx) return;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.setTransform(viewScale, 0, 0, viewScale, viewOffsetX, viewOffsetY);
  shapes.forEach(s => s.draw());
  // Draw lasso selection rect
  if (lassoRect) {
    ctx.strokeStyle = '#0d6efd';
    ctx.lineWidth   = 1 / viewScale;
    ctx.setLineDash([6 / viewScale, 3 / viewScale]);
    ctx.fillStyle   = 'rgba(13,110,253,0.08)';
    ctx.beginPath();
    ctx.rect(lassoRect.x, lassoRect.y, lassoRect.w, lassoRect.h);
    ctx.fill();
    ctx.stroke();
    ctx.setLineDash([]);
  }
  ctx.setTransform(1, 0, 0, 1, 0, 0);
}

// ── DOT generation ────────────────────────────────────────────
function _sanitize(name) {
  return (name || 'unnamed').replace(/[^a-zA-Z0-9_]/g, '_');
}


function shapesToBackendDOT() {
  function escapeHtml(s) {
    return (s || '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  const classNodes = shapes.filter(s => s.type === 'class' || s.type === 'interface');
  const connectors = shapes.filter(s => isConnectorShape(s.type));

  let dot = 'digraph JavaClasses {\n';
  dot += '    rankdir=TB;\n';
  dot += '    fontname="Arial";\n';
  dot += '    node [fontname="Arial"];\n';
  dot += '    edge [fontname="Arial", fontsize=10];\n\n';

  // ── Class/Interface nodes with HTML labels ─────────────────
  // Matches backend build_html_label exactly.
  classNodes.forEach(cls => {
    const rawName = cls.text || `node${cls.id}`;
    const clsId   = _sanitize(rawName);

    let classLabel = rawName;
    if (cls.type === 'interface') classLabel = `${rawName} (interface)`;
    else if (cls.isAbstract)     classLabel = `${rawName} (abstract)`;

    const publicMethods  = cls.methods.filter(m => m.visibility !== 'private');
    const privateMethods = cls.methods.filter(m => m.visibility === 'private');
    const publicFields   = cls.fields.filter(f => f.visibility !== 'private');
    const privateFields  = cls.fields.filter(f => f.visibility === 'private');

    let html = '<TABLE BORDER="0" CELLBORDER="0" CELLSPACING="2" CELLPADDING="4">';
    html += `<TR><TD><B><FONT POINT-SIZE="14">${escapeHtml(classLabel)}</FONT></B></TD></TR>`;

    publicMethods.forEach(m => {
      html += `<TR><TD><U>${escapeHtml(Shape._methodLabel(m))}</U></TD></TR>`;
    });
    publicFields.forEach(f => {
      html += `<TR><TD BORDER="1" BGCOLOR="lightyellow">${escapeHtml(Shape._fieldLabel(f))}</TD></TR>`;
    });

    const hasPublic  = publicMethods.length > 0 || publicFields.length > 0;
    const hasPrivate = privateMethods.length > 0 || privateFields.length > 0;
    if (hasPublic && hasPrivate) html += '<HR/>';

    privateFields.forEach(f => {
      html += `<TR><TD BORDER="1" BGCOLOR="lightyellow">${escapeHtml(Shape._fieldLabel(f))}</TD></TR>`;
    });
    privateMethods.forEach(m => {
      html += `<TR><TD><U>${escapeHtml(Shape._methodLabel(m))}</U></TD></TR>`;
    });

    html += '</TABLE>';

    const shape = cls.type === 'interface' ? 'diamond' : 'circle';
    dot += `    "${clsId}_class" [shape=${shape}, label=<${html}>, style=filled, fillcolor=white];\n`;
  });
  dot += '\n';

  // ── Inter-class relationships ──────────────────────────────
  connectors.forEach(conn => {
    if (!conn.startNode || !conn.endNode) return;
    const fromId = `${_sanitize(conn.startNode.text || `node${conn.startNode.id}`)}_class`;
    const toId   = `${_sanitize(conn.endNode.text   || `node${conn.endNode.id}`)}_class`;

    if (conn.type === 'extends') {
      dot += `    "${fromId}" -> "${toId}" [arrowhead=empty, style=solid, label=extends];\n`;
    } else if (conn.type === 'implements') {
      dot += `    "${fromId}" -> "${toId}" [arrowhead=empty, style=dashed, label=implements];\n`;
    } else if (conn.type === 'calls') {
      const labelAttr = conn.label ? `, label="${conn.label.replace(/"/g, '\\"')}"` : '';
      dot += `    "${fromId}" -> "${toId}" [arrowhead=normal, style=solid, color=blue${labelAttr}];\n`;
    }
  });

  dot += '}\n';
  return dot;
}

function updateDOTPreview() {
  const el = document.getElementById('dotPreview');
  if (el) el.textContent = shapesToBackendDOT();
}

// ── Property editor ───────────────────────────────────────────
function updatePropertyEditor() {
  const el = document.getElementById('propertyEditorContent');
  if (!el) return;

  if (selectedShapes.length > 0) {
    el.innerHTML = `<p class="text-muted"><small>${selectedShapes.length} shapes selected</small></p>
      <button class="btn btn-danger btn-sm w-100 mt-2" onclick="deleteSelectedGroup()">Delete Group</button>`;
    return;
  }
  if (!selectedShape) {
    el.innerHTML = '<p class="text-muted"><small>Select a shape to edit its properties</small></p>';
    return;
  }

  const s = selectedShape;
  let html = `<div class="mb-2"><span class="badge bg-secondary">${s.type}</span></div>`;

  if (s.type === 'class' || s.type === 'interface') {
    // ── Name ──
    html += `<div class="mb-2">
      <label class="form-label form-label-sm fw-semibold">Name</label>
      <input type="text" class="form-control form-control-sm"
             value="${(s.text || '').replace(/"/g, '&quot;')}"
             oninput="updateProp('text', this.value)">
    </div>`;

    if (s.type === 'class') {
      html += `<div class="form-check mb-2">
        <input class="form-check-input" type="checkbox" id="cb_abstract"
               ${s.isAbstract ? 'checked' : ''} onchange="updateProp('isAbstract', this.checked)">
        <label class="form-check-label small" for="cb_abstract">Abstract</label>
      </div>`;
    }

    // ── Methods ──
    html += `<div class="mb-1 d-flex align-items-center">
      <span class="fw-semibold small flex-grow-1">Methods</span>
      <button class="btn btn-outline-primary btn-sm py-0 px-1" style="font-size:11px;" onclick="addMethod()">+ Add</button>
    </div>`;

    s.methods.forEach((m, i) => {
      const visOpts = ['public','private','protected']
        .map(v => `<option value="${v}" ${m.visibility===v?'selected':''}>${v}</option>`).join('');
      html += `<div class="border rounded p-1 mb-1" style="font-size:11px;background:#f8f9fa;">
        <div class="d-flex gap-1 mb-1 align-items-center">
          <select class="form-select form-select-sm py-0" style="width:86px;font-size:11px;"
                  onchange="updateMethod(${i},'visibility',this.value)">${visOpts}</select>
          <input type="text" class="form-control form-control-sm py-0" style="font-size:11px;" placeholder="name"
                 value="${(m.name||'').replace(/"/g,'&quot;')}"
                 oninput="updateMethod(${i},'name',this.value)">
          <button class="btn btn-danger btn-sm py-0 px-1" style="font-size:11px;" onclick="removeMethod(${i})">✕</button>
        </div>
        <div class="d-flex gap-1 mb-1">
          <input type="text" class="form-control form-control-sm py-0" style="font-size:11px;flex:2;" placeholder="params"
                 value="${(m.params||'').replace(/"/g,'&quot;')}"
                 oninput="updateMethod(${i},'params',this.value)">
          <input type="text" class="form-control form-control-sm py-0" style="font-size:11px;flex:1;" placeholder="return"
                 value="${(m.returnType||'void').replace(/"/g,'&quot;')}"
                 oninput="updateMethod(${i},'returnType',this.value)">
        </div>
        <div class="d-flex gap-2">
          <div class="form-check form-check-inline mb-0">
            <input class="form-check-input" type="checkbox" id="ms${i}" ${m.isStatic?'checked':''}
                   onchange="updateMethod(${i},'isStatic',this.checked)">
            <label class="form-check-label" style="font-size:11px;" for="ms${i}">static</label>
          </div>
          <div class="form-check form-check-inline mb-0">
            <input class="form-check-input" type="checkbox" id="ma${i}" ${m.isAbstract?'checked':''}
                   onchange="updateMethod(${i},'isAbstract',this.checked)">
            <label class="form-check-label" style="font-size:11px;" for="ma${i}">abstract</label>
          </div>
        </div>
      </div>`;
    });

    // ── Variables ──
    html += `<div class="mb-1 d-flex align-items-center mt-2">
      <span class="fw-semibold small flex-grow-1">Variables</span>
      <button class="btn btn-outline-primary btn-sm py-0 px-1" style="font-size:11px;" onclick="addField()">+ Add</button>
    </div>`;

    s.fields.forEach((f, i) => {
      const visOpts = ['public','private','protected']
        .map(v => `<option value="${v}" ${f.visibility===v?'selected':''}>${v}</option>`).join('');
      html += `<div class="border rounded p-1 mb-1" style="font-size:11px;background:#ffffe0;">
        <div class="d-flex gap-1 mb-1 align-items-center">
          <select class="form-select form-select-sm py-0" style="width:86px;font-size:11px;"
                  onchange="updateField(${i},'visibility',this.value)">${visOpts}</select>
          <input type="text" class="form-control form-control-sm py-0" style="font-size:11px;flex:1;" placeholder="type"
                 value="${(f.fieldType||'').replace(/"/g,'&quot;')}"
                 oninput="updateField(${i},'fieldType',this.value)">
          <input type="text" class="form-control form-control-sm py-0" style="font-size:11px;flex:1;" placeholder="name"
                 value="${(f.name||'').replace(/"/g,'&quot;')}"
                 oninput="updateField(${i},'name',this.value)">
          <button class="btn btn-danger btn-sm py-0 px-1" style="font-size:11px;" onclick="removeField(${i})">✕</button>
        </div>
        <div class="d-flex gap-2">
          <div class="form-check form-check-inline mb-0">
            <input class="form-check-input" type="checkbox" id="fs${i}" ${f.isStatic?'checked':''}
                   onchange="updateField(${i},'isStatic',this.checked)">
            <label class="form-check-label" style="font-size:11px;" for="fs${i}">static</label>
          </div>
          <div class="form-check form-check-inline mb-0">
            <input class="form-check-input" type="checkbox" id="ff${i}" ${f.isFinal?'checked':''}
                   onchange="updateField(${i},'isFinal',this.checked)">
            <label class="form-check-label" style="font-size:11px;" for="ff${i}">final</label>
          </div>
        </div>
      </div>`;
    });

  } else if (isConnectorShape(s.type)) {
    if (s.type === 'calls') {
      html += `<p class="text-muted small mb-2">Blue solid arrow — pointer / method call</p>
      <div class="mb-2">
        <label class="form-label form-label-sm fw-semibold">Label <small class="text-muted">(e.g. main -&gt; add)</small></label>
        <input type="text" class="form-control form-control-sm"
               placeholder="e.g. main -> add"
               value="${(s.label||'').replace(/"/g,'&quot;')}"
               oninput="updateProp('label', this.value)">
      </div>`;
    } else {
      const desc = s.type === 'extends'    ? 'Solid line → hollow arrowhead (inheritance)'
                 : 'Dashed line → hollow arrowhead (interface implementation)';
      html += `<p class="text-muted small mb-2">${desc}</p>`;
    }
    html += `<div class="form-check mb-1">
      <input class="form-check-input" type="checkbox" id="cb_curved"
             ${s.curved ? 'checked' : ''} onchange="updateProp('curved', this.checked)">
      <label class="form-check-label small" for="cb_curved">Curved</label>
    </div>
    <div class="mb-2">
      <label class="form-label form-label-sm">Curve offset <small class="text-muted">(0 = auto)</small></label>
      <input type="range" class="form-range" min="-200" max="200" step="10"
             value="${s.curveOffset}"
             oninput="updateProp('curveOffset', +this.value)">
    </div>`;
  }

  html += `<button class="btn btn-danger btn-sm w-100 mt-2" onclick="deleteSelectedShape()">Delete</button>`;
  el.innerHTML = html;
}

// ── Method / Field management ─────────────────────────────────
function addMethod() {
  if (!selectedShape) return;
  selectedShape.methods.push({ visibility: 'public', isStatic: false, isAbstract: false, name: 'method', params: '', returnType: 'void' });
  selectedShape.autoResizeForContent();
  updatePropertyEditor();
  redraw();
  updateDOTPreview();
}

function removeMethod(i) {
  if (!selectedShape) return;
  selectedShape.methods.splice(i, 1);
  selectedShape.autoResizeForContent();
  updatePropertyEditor();
  redraw();
  updateDOTPreview();
}

function updateMethod(i, prop, value) {
  if (!selectedShape) return;
  selectedShape.methods[i][prop] = value;
  redraw();
  updateDOTPreview();
}

function addField() {
  if (!selectedShape) return;
  selectedShape.fields.push({ visibility: 'private', isStatic: false, isFinal: false, fieldType: '', name: 'field' });
  selectedShape.autoResizeForContent();
  updatePropertyEditor();
  redraw();
  updateDOTPreview();
}

function removeField(i) {
  if (!selectedShape) return;
  selectedShape.fields.splice(i, 1);
  selectedShape.autoResizeForContent();
  updatePropertyEditor();
  redraw();
  updateDOTPreview();
}

function updateField(i, prop, value) {
  if (!selectedShape) return;
  selectedShape.fields[i][prop] = value;
  redraw();
  updateDOTPreview();
}

function updateProp(prop, value) {
  if (!selectedShape) return;
  selectedShape[prop] = value;
  redraw();
  updateDOTPreview();
}

function deleteSelectedGroup() {
  if (selectedShapes.length === 0) return;
  const toDelete = new Set(selectedShapes);
  shapes = shapes.filter(s => {
    if (toDelete.has(s)) return false;
    if (isConnectorShape(s.type) && (toDelete.has(s.startNode) || toDelete.has(s.endNode))) return false;
    return true;
  });
  selectedShapes = [];
  updatePropertyEditor();
  redraw();
  updateDOTPreview();
}

function deleteSelectedShape() {
  if (!selectedShape) return;
  // Also remove any connectors attached to this shape
  shapes = shapes.filter(s => {
    if (s === selectedShape) return false;
    if (isConnectorShape(s.type) && (s.startNode === selectedShape || s.endNode === selectedShape)) return false;
    return true;
  });
  selectedShape = null;
  updatePropertyEditor();
  redraw();
  updateDOTPreview();
}

function clearCanvas() {
  if (!confirm('Clear the canvas?')) return;
  shapes = [];
  selectedShape  = null;
  selectedShapes = [];
  updatePropertyEditor();
  redraw();
  updateDOTPreview();
}

function exportToDOT() {
  const dot  = shapesToBackendDOT();
  const blob = new Blob([dot], { type: 'text/plain' });
  const url  = URL.createObjectURL(blob);
  const a    = document.createElement('a');
  a.href     = url;
  a.download = 'diagram.dot';
  a.click();
  URL.revokeObjectURL(url);
}

function exportToImage(format) {
  const mimeType = format === 'jpeg' ? 'image/jpeg' : 'image/png';
  const quality  = format === 'jpeg' ? 0.92 : undefined;

  // Composite the live canvas onto a white background (required for JPEG, good for PNG)
  const snapshot = document.createElement('canvas');
  snapshot.width  = canvas.width;
  snapshot.height = canvas.height;
  const snapCtx = snapshot.getContext('2d');
  snapCtx.fillStyle = '#ffffff';
  snapCtx.fillRect(0, 0, snapshot.width, snapshot.height);
  snapCtx.drawImage(canvas, 0, 0);

  const a   = document.createElement('a');
  a.href     = snapshot.toDataURL(mimeType, quality);
  a.download = `diagram.${format}`;
  a.click();
}

// ── Keyboard shortcuts ────────────────────────────────────────
document.addEventListener('keydown', e => {
  const tag = document.activeElement?.tagName?.toLowerCase();
  const isEditingText = tag === 'input' || tag === 'textarea' || tag === 'select'
                     || document.activeElement?.isContentEditable;
  if (isEditingText) return;

  if (e.key === 'Delete' || e.key === 'Backspace') {
    if (selectedShapes.length > 0) { e.preventDefault(); deleteSelectedGroup(); }
    else if (selectedShape)        { e.preventDefault(); deleteSelectedShape(); }
  }
  if (e.key === 'Escape') {
    drawingConnection = false;
    startConnectionPoint = null;
    isDrawing = false;
    redraw();
  }
  // Ctrl+0 = reset view
  if (e.key === '0' && e.ctrlKey) {
    e.preventDefault();
    resetView();
  }
});


// ── Diagram Comparer integration ───────────────────────────────────────
window.getCurrentDiagram = function () {
  const classNodes = shapes.filter(s => s.type === 'class' || s.type === 'interface');
  const connectors = shapes.filter(s => isConnectorShape(s.type));

  const classes = classNodes.map(cls => ({
    name:    cls.text || `node${cls.id}`,
    methods: cls.methods.map(m => ({ name: m.name || '' })),
    fields:  cls.fields.map(f  => ({ name: f.name  || '' })),
  }));

  const relationships = connectors
    .filter(c => c.startNode && c.endNode)
    .map(c => ({
      from: c.startNode.text || `node${c.startNode.id}`,
      to:   c.endNode.text   || `node${c.endNode.id}`,
      kind: c.type === 'extends'    ? 'Extends'
          : c.type === 'implements' ? 'Implements'
          : c.type === 'calls'      ? 'Calls'
          : 'Uses'
    }));

  return { classes, relationships };
};
