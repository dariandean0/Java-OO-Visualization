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

// Shape role groups
const NODE_SHAPES      = ['class', 'interface', 'class-header', 'field', 'method-public', 'method-private', 'method-protected'];
const CONNECTOR_SHAPES = ['extends', 'implements', 'calls', 'member-link'];

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

    // Java OO metadata
    this.visibility  = 'public';   // 'public' | 'private' | 'protected' | 'package'
    this.isStatic    = false;
    this.isFinal     = false;
    this.isAbstract  = false;
    this.fieldType   = '';         // e.g. "String"
    this.returnType  = 'void';     // for methods
    this.params      = '';         // e.g. "String name, int age"

    // Connector anchors
    this.startNode  = null;
    this.startPoint = null;   // 'north' | 'south' | 'east' | 'west'
    this.endNode    = null;
    this.endPoint   = null;
  }

  // ── Label builders ────────────────────────────────────────
  buildFieldLabel() {
    const parts = [];
    if (this.visibility && this.visibility !== 'package') parts.push(this.visibility);
    if (this.isStatic)  parts.push('static');
    if (this.isFinal)   parts.push('final');
    if (this.fieldType) parts.push(this.fieldType);
    parts.push(this.text || 'field');
    return parts.join(' ');
  }

  buildMethodLabel() {
    const mods = [];
    if (this.visibility && this.visibility !== 'package') mods.push(this.visibility);
    if (this.isStatic)   mods.push('static');
    if (this.isAbstract) mods.push('abstract');
    const prefix = mods.join(' ');
    const name   = this.text   || 'method';
    const params = this.params || '';
    const ret    = this.returnType || 'void';
    return `${prefix}${prefix ? ' ' : ''}${name}(${params}): ${ret}`;
  }

  // ── Connection points (N/S/E/W) ───────────────────────────
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

  getConnectionCoordinates(point) {
    const pts = this.getConnectionPoints();
    return pts[point] || { x: this.x + this.width / 2, y: this.y + this.height / 2 };
  }

  findConnectionPoint(x, y) {
    if (!isNodeShape(this.type)) return null;
    const pts = this.getConnectionPoints();
    for (const [dir, pt] of Object.entries(pts)) {
      if (Math.hypot(x - pt.x, y - pt.y) < 12) return dir;
    }
    return null;
  }

  drawConnectionPoints(highlight = false) {
    if (!isNodeShape(this.type)) return;
    const pts = this.getConnectionPoints();
    Object.entries(pts).forEach(([dir, pt]) => {
      ctx.beginPath();
      ctx.arc(pt.x, pt.y, 5, 0, 2 * Math.PI);
      ctx.fillStyle =
        (hoveredConnectionPoint?.shape === this && hoveredConnectionPoint?.point === dir)
          ? '#0d6efd'
          : highlight ? '#6c757d' : '#adb5bd';
      ctx.fill();
      ctx.strokeStyle = '#495057';
      ctx.lineWidth   = 1;
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

  _midLabel(sx, sy, ex, ey, label) {
    ctx.font         = '11px Arial';
    ctx.fillStyle    = '#444';
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(label, (sx + ex) / 2, (sy + ey) / 2 - 10);
  }

  // ── Main draw dispatcher ──────────────────────────────────
  draw() {
    ctx.lineWidth   = selectedShape === this ? this.lineWidth + 1 : this.lineWidth;
    ctx.strokeStyle = this.strokeColor;
    if (selectedShape === this) { ctx.shadowColor = '#0d6efd'; ctx.shadowBlur = 10; }

    switch (this.type) {
      case 'class':            this._drawClass();          break;
      case 'interface':        this._drawInterface();      break;
      case 'class-header':     this._drawClassHeader();    break;
      case 'field':            this._drawField();          break;
      case 'method-public':
      case 'method-private':
      case 'method-protected': this._drawMethod();         break;
      case 'extends':          this._drawExtendsArrow();   break;
      case 'implements':       this._drawImplementsArrow();break;
      case 'calls':            this._drawCallsArrow();     break;
      case 'member-link':      this._drawMemberLink();     break;
    }

    ctx.shadowColor = 'transparent';
    ctx.shadowBlur  = 0;

    if (isNodeShape(this.type) && (selectedShape === this || isConnectorShape(currentTool))) {
      this.drawConnectionPoints(selectedShape === this);
    }
  }

  // ── Node draw methods ─────────────────────────────────────
  _drawClass() {
    const HDR = 34;
    const name = this.text || 'ClassName';
    const lw = selectedShape === this ? 3 : 2;

    // Background
    ctx.fillStyle = '#f0f0f0';
    ctx.fillRect(this.x, this.y, this.width, this.height);
    // Header bar
    ctx.fillStyle = '#add8e6';
    ctx.fillRect(this.x, this.y, this.width, HDR);
    // Border
    ctx.strokeStyle = this.strokeColor;
    ctx.lineWidth = lw;
    ctx.strokeRect(this.x, this.y, this.width, this.height);
    ctx.beginPath();
    ctx.moveTo(this.x, this.y + HDR);
    ctx.lineTo(this.x + this.width, this.y + HDR);
    ctx.stroke();
    // Label
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle    = '#000';
    if (this.isAbstract) {
      ctx.font = '10px Arial'; ctx.fillText('«abstract»', this.x + this.width / 2, this.y + HDR / 2 - 7);
      ctx.font = 'italic bold 12px Arial';
    } else {
      ctx.font = 'bold 13px Arial';
    }
    ctx.fillText(name, this.x + this.width / 2, this.y + (this.isAbstract ? HDR / 2 + 6 : HDR / 2));
  }

  _drawInterface() {
    const HDR = 34;
    const name = this.text || 'InterfaceName';
    const lw = selectedShape === this ? 3 : 2;

    ctx.fillStyle = '#f5f8ff';
    ctx.fillRect(this.x, this.y, this.width, this.height);
    ctx.fillStyle = '#c4deff';
    ctx.fillRect(this.x, this.y, this.width, HDR);
    ctx.strokeStyle = this.strokeColor;
    ctx.lineWidth = lw;
    ctx.setLineDash([6, 3]);
    ctx.strokeRect(this.x, this.y, this.width, this.height);
    ctx.setLineDash([]);
    ctx.beginPath();
    ctx.moveTo(this.x, this.y + HDR);
    ctx.lineTo(this.x + this.width, this.y + HDR);
    ctx.stroke();
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle    = '#000';
    ctx.font = '10px Arial'; ctx.fillText('«interface»', this.x + this.width / 2, this.y + HDR / 2 - 7);
    ctx.font = 'italic bold 12px Arial';
    ctx.fillText(name, this.x + this.width / 2, this.y + HDR / 2 + 7);
  }

  _drawField() {
    const EAR = 10;
    const lbl = this.buildFieldLabel();
    ctx.fillStyle = '#ffffe0';
    ctx.beginPath();
    ctx.moveTo(this.x, this.y);
    ctx.lineTo(this.x + this.width - EAR, this.y);
    ctx.lineTo(this.x + this.width, this.y + EAR);
    ctx.lineTo(this.x + this.width, this.y + this.height);
    ctx.lineTo(this.x, this.y + this.height);
    ctx.closePath();
    ctx.fill();
    ctx.strokeStyle = this.strokeColor;
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(this.x + this.width - EAR, this.y);
    ctx.lineTo(this.x + this.width - EAR, this.y + EAR);
    ctx.lineTo(this.x + this.width, this.y + EAR);
    ctx.stroke();
    ctx.font = '11px monospace';
    ctx.fillStyle    = '#000';
    ctx.textAlign    = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(lbl, this.x + 6, this.y + this.height / 2, this.width - 20);
  }

  _drawMethod() {
    const TAB_W = 8, TAB_H = 6;
    const lbl = this.buildMethodLabel();
    const fill = this.type === 'method-public'  ? '#90ee90'
               : this.type === 'method-private' ? '#f08080'
               : '#d3d3d3';  // protected
    ctx.fillStyle = fill;
    ctx.fillRect(this.x + TAB_W, this.y, this.width - TAB_W, this.height);
    ctx.strokeStyle = this.strokeColor;
    ctx.strokeRect(this.x + TAB_W, this.y, this.width - TAB_W, this.height);
    // Left tab notches (component shape)
    const topTabY = this.y + Math.floor(this.height * 0.25);
    const botTabY = this.y + Math.floor(this.height * 0.60);
    ctx.fillRect(this.x, topTabY, TAB_W, TAB_H);
    ctx.strokeRect(this.x, topTabY, TAB_W, TAB_H);
    ctx.fillRect(this.x, botTabY, TAB_W, TAB_H);
    ctx.strokeRect(this.x, botTabY, TAB_W, TAB_H);
    ctx.font = '11px monospace';
    ctx.fillStyle    = '#000';
    ctx.textAlign    = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(lbl, this.x + TAB_W + 5, this.y + this.height / 2, this.width - TAB_W - 10);
  }

  // ── Class-header ellipse node ─────────────────────────────
  _drawClassHeader() {
    const lbl = this.text || 'ClassName';
    const cx  = this.x + this.width  / 2;
    const cy  = this.y + this.height / 2;
    const rx  = this.width  / 2;
    const ry  = this.height / 2;
    ctx.beginPath();
    ctx.ellipse(cx, cy, rx, ry, 0, 0, 2 * Math.PI);
    ctx.fillStyle   = '#add8e6';
    ctx.fill();
    ctx.strokeStyle = selectedShape === this ? '#0d6efd' : this.strokeColor;
    ctx.lineWidth   = selectedShape === this ? 3 : 2;
    ctx.stroke();
    ctx.font         = 'bold 12px Arial';
    ctx.fillStyle    = '#000';
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(lbl, cx, cy, this.width - 10);
  }

  // ── Connector draw methods ────────────────────────────────
  _drawMemberLink() {
    const { startX, startY, endX, endY } = this.getConnectorCoords();
    ctx.setLineDash([7, 4]);
    ctx.strokeStyle = selectedShape === this ? '#0d6efd' : '#555';
    ctx.lineWidth   = selectedShape === this ? 3 : 1.5;
    ctx.beginPath();
    ctx.moveTo(startX, startY);
    ctx.lineTo(endX, endY);
    ctx.stroke();
    ctx.setLineDash([]);
  }

  _drawExtendsArrow() {
    const { startX, startY, endX, endY } = this.getConnectorCoords();
    const LEN = 18;
    const angle = Math.atan2(endY - startY, endX - startX);
    ctx.setLineDash([]);
    ctx.strokeStyle = this.strokeColor;
    ctx.lineWidth   = selectedShape === this ? 3 : 2;
    ctx.beginPath();
    ctx.moveTo(startX, startY);
    ctx.lineTo(endX - LEN * Math.cos(angle), endY - LEN * Math.sin(angle));
    ctx.stroke();
    this._hollowArrowhead(endX, endY, angle, LEN);
    this._midLabel(startX, startY, endX, endY, 'extends');
  }

  _drawImplementsArrow() {
    const { startX, startY, endX, endY } = this.getConnectorCoords();
    const LEN = 18;
    const angle = Math.atan2(endY - startY, endX - startX);
    ctx.setLineDash([8, 4]);
    ctx.strokeStyle = this.strokeColor;
    ctx.lineWidth   = selectedShape === this ? 3 : 2;
    ctx.beginPath();
    ctx.moveTo(startX, startY);
    ctx.lineTo(endX - LEN * Math.cos(angle), endY - LEN * Math.sin(angle));
    ctx.stroke();
    ctx.setLineDash([]);
    this._hollowArrowhead(endX, endY, angle, LEN);
    this._midLabel(startX, startY, endX, endY, 'implements');
  }

  _drawCallsArrow() {
    const { startX, startY, endX, endY } = this.getConnectorCoords();
    const LEN = 14;
    const angle = Math.atan2(endY - startY, endX - startX);
    ctx.setLineDash([]);
    ctx.strokeStyle = '#0000cd';
    ctx.lineWidth   = selectedShape === this ? 3 : 2;
    ctx.beginPath();
    ctx.moveTo(startX, startY);
    ctx.lineTo(endX, endY);
    ctx.stroke();
    this._filledArrowhead(endX, endY, angle, LEN, '#0000cd');
    this._midLabel(startX, startY, endX, endY, 'calls');
  }

  // ── Hit testing ───────────────────────────────────────────
  contains(x, y) {
    if (isConnectorShape(this.type)) return this._containsLine(x, y);
    return x >= this.x && x <= this.x + this.width &&
           y >= this.y && y <= this.y + this.height;
  }

  _containsLine(x, y) {
    const { startX, startY, endX, endY } = this.getConnectorCoords();
    return this._ptLineDist(x, y, startX, startY, endX, endY) < 8;
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
    if (canvas) canvas.style.cursor = currentTool === 'select' ? 'default' : 'crosshair';
  });
});

// ── Canvas event handlers ─────────────────────────────────────
if (canvas) {
  canvas.addEventListener('mousedown', handleMouseDown);
  canvas.addEventListener('mousemove', handleMouseMove);
  canvas.addEventListener('mouseup',   handleMouseUp);
  canvas.addEventListener('dblclick',  handleDoubleClick);
}

function handleMouseDown(e) {
  const rect = canvas.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;

  if (currentTool === 'select') {
    for (let i = shapes.length - 1; i >= 0; i--) {
      if (shapes[i].contains(x, y)) {
        selectedShape  = shapes[i];
        isDragging     = true;
        dragOffsetX    = x - selectedShape.x;
        dragOffsetY    = y - selectedShape.y;
        updatePropertyEditor();
        redraw();
        return;
      }
    }
    selectedShape = null;
    updatePropertyEditor();
    redraw();

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
  const rect = canvas.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;

  // Update hovered connection point
  hoveredConnectionPoint = null;
  if (isConnectorShape(currentTool) || currentTool === 'select') {
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

  if (isDragging && selectedShape && isNodeShape(selectedShape.type)) {
    selectedShape.x = x - dragOffsetX;
    selectedShape.y = y - dragOffsetY;
    redraw();
    updateDOTPreview();
  } else if (drawingConnection || isDrawing) {
    redraw();
    ctx.strokeStyle = '#888';
    ctx.lineWidth   = 1;
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(startX, startY);
    ctx.lineTo(x, y);
    ctx.stroke();
    ctx.setLineDash([]);
  }
}

function handleMouseUp(e) {
  const rect = canvas.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;

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
      const isLarge = currentTool === 'class' || currentTool === 'interface';
      let dw = Math.abs(x - startX);
      let dh = Math.abs(y - startY);
      if (dw < 15) dw = isLarge ? 200 : 200;
      if (dh < 15) dh = isLarge ? 150 : 36;

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
  if (!selectedShape || !isNodeShape(selectedShape.type)) return;
  const lbl = selectedShape.type.startsWith('method') ? 'Method name:'
            : selectedShape.type === 'field'           ? 'Field name:'
            : 'Class / Interface name:';
  const text = prompt(lbl, selectedShape.text);
  if (text !== null) {
    selectedShape.text = text;
    updatePropertyEditor();
    updateDOTPreview();
    redraw();
  }
}

function redraw() {
  if (!ctx) return;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  shapes.forEach(s => s.draw());
}

// ── DOT generation ────────────────────────────────────────────
function _sanitize(name) {
  return (name || 'unnamed').replace(/[^a-zA-Z0-9_]/g, '_');
}

function _containedIn(inner, outer) {
  const cx = inner.x + inner.width  / 2;
  const cy = inner.y + inner.height / 2;
  return cx >= outer.x && cx <= outer.x + outer.width &&
         cy >= outer.y && cy <= outer.y + outer.height;
}

function shapesToBackendDOT() {
  const classNodes   = shapes.filter(s => s.type === 'class' || s.type === 'interface');
  const headerNodes  = shapes.filter(s => s.type === 'class-header');
  const fieldNodes   = shapes.filter(s => s.type === 'field');
  const methodNodes  = shapes.filter(s => s.type.startsWith('method-'));
  const connectors   = shapes.filter(s => isConnectorShape(s.type));
  const memberLinks  = connectors.filter(s => s.type === 'member-link');

  let dot = 'digraph JavaClasses {\n';
  dot += '    rankdir=TB;\n';
  dot += '    fontname="Arial";\n';
  dot += '    node [fontname="Arial"];\n';
  dot += '    edge [fontname="Arial", fontsize=10];\n\n';

  // ── Class/Interface subgraphs ──────────────────────────────
  classNodes.forEach(cls => {
    const rawName = cls.text || `node${cls.id}`;
    const clsId   = _sanitize(rawName);

    let classLabel = rawName;
    if (cls.type === 'interface') classLabel = `${rawName} (interface)`;
    else if (cls.isAbstract)     classLabel = `${rawName} (abstract)`;

    const containedFields   = fieldNodes.filter(f  => _containedIn(f,  cls));
    const containedMethods  = methodNodes.filter(m  => _containedIn(m,  cls));
    // Explicit class-header nodes placed inside this class box
    const containedHeaders  = headerNodes.filter(h  => _containedIn(h,  cls));
    // Use explicit header label if one is placed inside
    const hdrLabel = containedHeaders.length > 0
      ? (containedHeaders[0].text || classLabel)
      : classLabel;

    dot += `    subgraph cluster_${clsId} {\n`;
    dot += `        label="${classLabel}";\n`;
    dot += `        style=filled;\n`;
    dot += `        color=lightgrey;\n`;
    dot += `        node [shape=box, style=filled, fillcolor=white];\n\n`;

    dot += `        "${clsId}_class" [label="${hdrLabel}", shape=ellipse, style=filled, fillcolor=lightblue];\n`;

    // Fields subgraph
    if (containedFields.length > 0) {
      dot += `        subgraph cluster_${clsId}_fields {\n`;
      dot += `            label="Fields";\n`;
      dot += `            style=dashed;\n`;
      containedFields.forEach(f => {
        const fId  = `${clsId}_${_sanitize(f.text || `f${f.id}`)}`;
        const lbl  = f.buildFieldLabel();
        dot += `            "${fId}" [label="${lbl}", shape=note, style=filled, fillcolor=lightyellow];\n`;
      });
      dot += `        }\n`;
    }

    // Methods subgraph
    if (containedMethods.length > 0) {
      dot += `        subgraph cluster_${clsId}_methods {\n`;
      dot += `            label="Methods";\n`;
      dot += `            style=dashed;\n`;
      containedMethods.forEach(m => {
        const mId   = `${clsId}_${_sanitize(m.text || `m${m.id}`)}`;
        const lbl   = m.buildMethodLabel();
        const color = m.type === 'method-public'  ? 'lightgreen'
                    : m.type === 'method-private' ? 'lightcoral'
                    : 'lightgray';
        dot += `            "${mId}" [label="${lbl}", shape=component, style=filled, fillcolor=${color}];\n`;
      });
      dot += `        }\n`;
    }

    // Internal connections: class_header → each field / method.
    // If explicit member-links exist from a contained header, use those;
    // otherwise auto-connect all contained fields & methods.
    const containedHeaderSet = new Set(containedHeaders);
    const explicitLinks = memberLinks.filter(
      l => l.startNode && containedHeaderSet.has(l.startNode) && _containedIn(l.startNode, cls)
    );

    if (explicitLinks.length > 0) {
      // Only emit the member-links the user explicitly drew
      explicitLinks.forEach(l => {
        const targetNode = l.endNode;
        if (!targetNode) return;
        let targetId;
        if (targetNode.type === 'field') {
          targetId = `${clsId}_${_sanitize(targetNode.text || `f${targetNode.id}`)}`;
        } else if (targetNode.type.startsWith('method-')) {
          targetId = `${clsId}_${_sanitize(targetNode.text || `m${targetNode.id}`)}`;
        } else {
          targetId = `${_sanitize(targetNode.text || `node${targetNode.id}`)}_class`;
        }
        dot += `        "${clsId}_class" -> "${targetId}" [style=dashed, arrowhead=none];\n`;
      });
    } else {
      // Auto-connect all contained fields & methods
      containedFields.forEach(f => {
        const fId = `${clsId}_${_sanitize(f.text || `f${f.id}`)}`;
        dot += `        "${clsId}_class" -> "${fId}" [style=dashed, arrowhead=none];\n`;
      });
      containedMethods.forEach(m => {
        const mId = `${clsId}_${_sanitize(m.text || `m${m.id}`)}`;
        dot += `        "${clsId}_class" -> "${mId}" [style=dashed, arrowhead=none];\n`;
      });
    }

    dot += `    }\n\n`;
  });

  // ── Standalone class-header nodes not inside any class box ─
  headerNodes
    .filter(h => !classNodes.some(c => _containedIn(h, c)))
    .forEach(h => {
      const hId  = _sanitize(h.text || `hdr${h.id}`);
      const lbl  = h.text || `hdr${h.id}`;
      dot += `    "${hId}_class" [label="${lbl}", shape=ellipse, style=filled, fillcolor=lightblue];\n`;
    });

  // ── Standalone member-link edges not inside any class box ──
  memberLinks
    .filter(l => l.startNode && l.endNode)
    .filter(l => !classNodes.some(c => _containedIn(l.startNode, c)))
    .forEach(l => {
      const fromId = l.startNode.type === 'class-header'
        ? `${_sanitize(l.startNode.text || `hdr${l.startNode.id}`)}_class`
        : _sanitize(l.startNode.text || `node${l.startNode.id}`);
      const toNode = l.endNode;
      let toId;
      if (toNode.type === 'field') {
        const parent = classNodes.find(c => _containedIn(toNode, c));
        const pName  = parent ? _sanitize(parent.text || `node${parent.id}`) : 'unknown';
        toId = `${pName}_${_sanitize(toNode.text || `f${toNode.id}`)}`;
      } else if (toNode.type.startsWith('method-')) {
        const parent = classNodes.find(c => _containedIn(toNode, c));
        const pName  = parent ? _sanitize(parent.text || `node${parent.id}`) : 'unknown';
        toId = `${pName}_${_sanitize(toNode.text || `m${toNode.id}`)}`;
      } else {
        toId = `${_sanitize(toNode.text || `node${toNode.id}`)}_class`;
      }
      dot += `    "${fromId}" -> "${toId}" [style=dashed, arrowhead=none];\n`;
    });

  // ── Inter-class relationships ──────────────────────────────
  connectors
    .filter(conn => conn.type !== 'member-link')
    .forEach(conn => {
      if (!conn.startNode || !conn.endNode) return;

      if (conn.type === 'extends' || conn.type === 'implements') {
        const fromId = `${_sanitize(conn.startNode.text || `node${conn.startNode.id}`)}_class`;
        const toId   = `${_sanitize(conn.endNode.text   || `node${conn.endNode.id}`)}_class`;
        if (conn.type === 'extends') {
          dot += `    "${fromId}" -> "${toId}" [arrowhead=empty, style=solid, label=extends];\n`;
        } else {
          dot += `    "${fromId}" -> "${toId}" [arrowhead=empty, style=dashed, label=implements];\n`;
        }

      } else if (conn.type === 'calls') {
        function methodNodeId(node) {
          const parent = classNodes.find(c => _containedIn(node, c));
          const pName  = parent ? _sanitize(parent.text || `node${parent.id}`) : _sanitize(node.text || `node${node.id}`);
          return `${pName}_${_sanitize(node.text || `m${node.id}`)}`;
        }
        const fromIsMethod = conn.startNode.type.startsWith('method-');
        const toIsMethod   = conn.endNode.type.startsWith('method-');
        const fromId = fromIsMethod ? methodNodeId(conn.startNode) : `${_sanitize(conn.startNode.text || `node${conn.startNode.id}`)}_class`;
        const toId   = toIsMethod   ? methodNodeId(conn.endNode)   : `${_sanitize(conn.endNode.text   || `node${conn.endNode.id}`)}_class`;
        dot += `    "${fromId}" -> "${toId}" [arrowhead=normal, style=solid, color=blue];\n`;
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
  if (!selectedShape) {
    el.innerHTML = '<p class="text-muted"><small>Select a shape to edit its properties</small></p>';
    return;
  }
  const s = selectedShape;
  let html = `<div class="mb-2"><span class="badge bg-secondary">${s.type}</span></div>`;

  if (s.type === 'class' || s.type === 'interface') {
    html += propText('Name', s.text || '', 'text');
    if (s.type === 'class') html += propCheck('Abstract', 'isAbstract', s.isAbstract);

  } else if (s.type === 'class-header') {
    html += propText('Label', s.text || '', 'text');

  } else if (s.type === 'field') {
    html += propText('Field Name', s.text || '', 'text');
    html += propText('Type', s.fieldType || '', 'fieldType', 'e.g. String, int');
    html += propVis(s.visibility);
    html += propCheck('static', 'isStatic', s.isStatic);
    html += propCheck('final',  'isFinal',  s.isFinal);

  } else if (s.type.startsWith('method-')) {
    html += propText('Method Name', s.text || '', 'text');
    html += propText('Return Type', s.returnType || 'void', 'returnType', 'e.g. void, String');
    html += propText('Parameters', s.params || '', 'params', 'e.g. String name, int age');
    html += propCheck('static',   'isStatic',   s.isStatic);
    html += propCheck('abstract', 'isAbstract', s.isAbstract);

  } else if (isConnectorShape(s.type)) {
    const desc = s.type === 'extends'     ? 'Solid line → hollow arrowhead'
               : s.type === 'implements'  ? 'Dashed line → hollow arrowhead'
               : s.type === 'member-link' ? 'Dashed line, no arrowhead (class header → member)'
               : 'Solid blue line → filled arrowhead';
    html += `<p class="text-muted small mb-1">${desc}</p>`;
  }

  html += `<button class="btn btn-danger btn-sm w-100 mt-2" onclick="deleteSelectedShape()">Delete</button>`;
  el.innerHTML = html;
}

function propText(label, value, prop, placeholder = '') {
  return `<div class="mb-2">
    <label class="form-label form-label-sm fw-semibold">${label}</label>
    <input type="text" class="form-control form-control-sm" placeholder="${placeholder}"
           value="${value.replace(/"/g, '&quot;')}"
           oninput="updateProp('${prop}', this.value)">
  </div>`;
}
function propVis(current) {
  const opts = ['public', 'private', 'protected', 'package']
    .map(v => `<option value="${v}" ${current === v ? 'selected' : ''}>${v}</option>`)
    .join('');
  return `<div class="mb-2">
    <label class="form-label form-label-sm fw-semibold">Visibility</label>
    <select class="form-select form-select-sm" onchange="updateProp('visibility', this.value)">${opts}</select>
  </div>`;
}
function propCheck(label, prop, checked) {
  return `<div class="form-check mb-1">
    <input class="form-check-input" type="checkbox" id="cb_${prop}"
           ${checked ? 'checked' : ''} onchange="updateProp('${prop}', this.checked)">
    <label class="form-check-label" for="cb_${prop}">${label}</label>
  </div>`;
}

function updateProp(prop, value) {
  if (!selectedShape) return;
  selectedShape[prop] = value;
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
  selectedShape = null;
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

// ── Keyboard shortcuts ────────────────────────────────────────
document.addEventListener('keydown', e => {
  // Never intercept keys while the user is typing in a form field
  const tag = document.activeElement?.tagName?.toLowerCase();
  const isEditingText = tag === 'input' || tag === 'textarea' || tag === 'select'
                     || document.activeElement?.isContentEditable;
  if (isEditingText) return;

  if ((e.key === 'Delete' || e.key === 'Backspace') && selectedShape) {
    e.preventDefault();
    deleteSelectedShape();
  }
  if (e.key === 'Escape') {
    drawingConnection = false;
    startConnectionPoint = null;
    isDrawing = false;
    redraw();
  }
});

// ── Diagram Comparer integration ───────────────────────────────────────
window.getCurrentDiagram = function () {
  const classNodes = shapes.filter(s => s.type === 'class' || s.type === 'interface');
  const connectors = shapes.filter(s => isConnectorShape(s.type));

    const classes = classNodes.map(cls => {
    const containedMethods = shapes
      .filter(s => s.type.startsWith('method-') && _containedIn(s, cls));

    const linkedMethods = shapes
      .filter(s => isConnectorShape(s.type)
               && s.startNode === cls
               && s.endNode
               && s.endNode.type.startsWith('method-'))
      .map(s => s.endNode);

    const allMethodShapes = [...new Map(
      [...containedMethods, ...linkedMethods].map(m => [m.id, m])
    ).values()];

    const methods = allMethodShapes.map(m => ({
      name: m.text || '',
    }));

    const fields = shapes
      .filter(s => s.type === 'field' && _containedIn(s, cls))
      .map(f => ({
        name: f.text || '',
      }));

    return {
      name: cls.text || `node${cls.id}`,
      methods,
      fields,
    };
  });

  const relationships = connectors
    .filter(c => c.startNode && c.endNode)
    .filter(c => !c.endNode.type.startsWith('method-') && c.endNode.type !== 'field')
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
