// Diagram Creator Logic
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

// Shape class
class Shape {
   constructor(type, x, y, width, height, text = '') {
      this.id = nextShapeId++;
      this.type = type;
      this.x = x;
      this.y = y;
      this.width = width;
      this.height = height;
      this.text = text;
      this.strokeColor = '#000000';
      this.fillColor = '#ffffff';
      this.lineWidth = 2;
      // For connectors (line/arrow)
      this.startNode = null;
      this.startPoint = null; // 'north', 'south', 'east', 'west'
      this.endNode = null;
      this.endPoint = null;
   }
   
   getConnectionPoints() {
      if (this.type === 'line' || this.type === 'arrow' || this.type === 'text') {
         return [];
      }
      
      const centerX = this.x + this.width / 2;
      const centerY = this.y + this.height / 2;
      
      if (this.type === 'circle') {
         const radius = Math.min(this.width, this.height) / 2;
         return {
            north: { x: centerX, y: this.y },
            south: { x: centerX, y: this.y + this.height },
            east: { x: this.x + this.width, y: centerY },
            west: { x: this.x, y: centerY }
         };
      } else {
         return {
            north: { x: centerX, y: this.y },
            south: { x: centerX, y: this.y + this.height },
            east: { x: this.x + this.width, y: centerY },
            west: { x: this.x, y: centerY }
         };
      }
   }
   
   drawConnectionPoints(highlight = false) {
      if (this.type === 'line' || this.type === 'arrow' || this.type === 'text') {
         return;
      }
      
      const points = this.getConnectionPoints();
      const pointRadius = 5;
      
      Object.entries(points).forEach(([direction, point]) => {
         ctx.beginPath();
         ctx.arc(point.x, point.y, pointRadius, 0, 2 * Math.PI);
         
         if (hoveredConnectionPoint && 
             hoveredConnectionPoint.shape === this && 
             hoveredConnectionPoint.point === direction) {
            ctx.fillStyle = '#0d6efd';
         } else if (highlight) {
            ctx.fillStyle = '#6c757d';
         } else {
            ctx.fillStyle = '#dee2e6';
         }
         
         ctx.fill();
         ctx.strokeStyle = '#495057';
         ctx.lineWidth = 1;
         ctx.stroke();
      });
   }
   
   getConnectionCoordinates(point) {
      const points = this.getConnectionPoints();
      return points[point] || { x: this.x + this.width/2, y: this.y + this.height/2 };
   }
    
   draw() {
      ctx.strokeStyle = this.strokeColor;
      ctx.lineWidth = selectedShape === this ? this.lineWidth + 1 : this.lineWidth;
      ctx.fillStyle = this.fillColor;
        
      if (selectedShape === this) {
         ctx.shadowColor = '#0d6efd';
         ctx.shadowBlur = 10;
      }
        
      switch(this.type) {
         case 'circle':
            ctx.beginPath();
            const radius = Math.min(this.width, this.height) / 2;
            ctx.arc(this.x + this.width/2, this.y + this.height/2, radius, 0, 2 * Math.PI);
            ctx.fill();
            ctx.stroke();
            break;
         case 'rectangle':
            ctx.fillRect(this.x, this.y, this.width, this.height);
            ctx.strokeRect(this.x, this.y, this.width, this.height);
            break;
         case 'line':
            this.drawLine();
            break;
         case 'arrow':
            this.drawArrow();
            break;
         case 'text':
            ctx.font = '16px Arial';
            ctx.fillStyle = this.strokeColor;
            ctx.fillText(this.text, this.x, this.y);
            if (selectedShape === this) {
               const metrics = ctx.measureText(this.text);
               ctx.strokeStyle = '#0d6efd';
               ctx.strokeRect(this.x - 2, this.y - 16, metrics.width + 4, 20);
            }
            break;
      }
        
      ctx.shadowColor = 'transparent';
      ctx.shadowBlur = 0;
        
      // Draw text label for shapes
      if (this.text && this.type !== 'text' && this.type !== 'line' && this.type !== 'arrow') {
         ctx.font = '14px Arial';
         ctx.fillStyle = this.strokeColor;
         ctx.textAlign = 'center';
         ctx.textBaseline = 'middle';
         ctx.fillText(this.text, this.x + this.width/2, this.y + this.height/2);
      }
      
      // Draw connection points when selected or when drawing connections
      if (selectedShape === this || (currentTool === 'line' || currentTool === 'arrow')) {
         this.drawConnectionPoints(selectedShape === this);
      }
   }
   
   drawLine() {
      let startX, startY, endX, endY;
      
      if (this.startNode && this.startPoint) {
         const startCoords = this.startNode.getConnectionCoordinates(this.startPoint);
         startX = startCoords.x;
         startY = startCoords.y;
      } else {
         startX = this.x;
         startY = this.y;
      }
      
      if (this.endNode && this.endPoint) {
         const endCoords = this.endNode.getConnectionCoordinates(this.endPoint);
         endX = endCoords.x;
         endY = endCoords.y;
      } else {
         endX = this.x + this.width;
         endY = this.y + this.height;
      }
      
      ctx.beginPath();
      ctx.moveTo(startX, startY);
      ctx.lineTo(endX, endY);
      ctx.stroke();
   }
   
   drawArrow() {
      let startX, startY, endX, endY;
      
      if (this.startNode && this.startPoint) {
         const startCoords = this.startNode.getConnectionCoordinates(this.startPoint);
         startX = startCoords.x;
         startY = startCoords.y;
      } else {
         startX = this.x;
         startY = this.y;
      }
      
      if (this.endNode && this.endPoint) {
         const endCoords = this.endNode.getConnectionCoordinates(this.endPoint);
         endX = endCoords.x;
         endY = endCoords.y;
      } else {
         endX = this.x + this.width;
         endY = this.y + this.height;
      }
      
      const headLength = 15;
      const angle = Math.atan2(endY - startY, endX - startX);
         
      ctx.beginPath();
      ctx.moveTo(startX, startY);
      ctx.lineTo(endX, endY);
      ctx.stroke();
         
      ctx.beginPath();
      ctx.moveTo(endX, endY);
      ctx.lineTo(endX - headLength * Math.cos(angle - Math.PI / 6),
                  endY - headLength * Math.sin(angle - Math.PI / 6));
      ctx.lineTo(endX - headLength * Math.cos(angle + Math.PI / 6),
                  endY - headLength * Math.sin(angle + Math.PI / 6));
      ctx.closePath();
      ctx.fillStyle = this.strokeColor;
      ctx.fill();
   }
    
   contains(x, y) {
      if (this.type === 'line' || this.type === 'arrow') {
         return this.containsLine(x, y);
      } else if (this.type === 'circle') {
         const radius = Math.min(this.width, this.height) / 2;
         const dx = x - (this.x + this.width/2);
         const dy = y - (this.y + this.height/2);
         return Math.sqrt(dx*dx + dy*dy) <= radius;
      } else if (this.type === 'text') {
         ctx.font = '16px Arial';
         const metrics = ctx.measureText(this.text);
         return x >= this.x && x <= this.x + metrics.width && 
                  y >= this.y - 16 && y <= this.y + 4;
      } else {
         return x >= this.x && x <= this.x + this.width &&
                  y >= this.y && y <= this.y + this.height;
      }
   }
   
   containsLine(x, y) {
      let startX, startY, endX, endY;
      
      if (this.startNode && this.startPoint) {
         const startCoords = this.startNode.getConnectionCoordinates(this.startPoint);
         startX = startCoords.x;
         startY = startCoords.y;
      } else {
         startX = this.x;
         startY = this.y;
      }
      
      if (this.endNode && this.endPoint) {
         const endCoords = this.endNode.getConnectionCoordinates(this.endPoint);
         endX = endCoords.x;
         endY = endCoords.y;
      } else {
         endX = this.x + this.width;
         endY = this.y + this.height;
      }
      
      const distance = this.pointToLineDistance(x, y, startX, startY, endX, endY);
      return distance < 8;
   }
    
   pointToLineDistance(px, py, x1, y1, x2, y2) {
      const dx = x2 - x1;
      const dy = y2 - y1;
      const length = Math.sqrt(dx*dx + dy*dy);
      if (length === 0) return Math.sqrt((px-x1)**2 + (py-y1)**2);
        
      const t = Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / (length * length)));
      const projX = x1 + t * dx;
      const projY = y1 + t * dy;
      return Math.sqrt((px - projX)**2 + (py - projY)**2);
   }
   
   findConnectionPoint(x, y) {
      if (this.type === 'line' || this.type === 'arrow' || this.type === 'text') {
         return null;
      }
      
      const points = this.getConnectionPoints();
      const threshold = 10;
      
      for (let [direction, point] of Object.entries(points)) {
         const dist = Math.sqrt((x - point.x)**2 + (y - point.y)**2);
         if (dist < threshold) {
            return direction;
         }
      }
      
      return null;
   }
}


// Tool button handlers
document.querySelectorAll('.tool-button').forEach(button => {
   button.addEventListener('click', function() {
      document.querySelectorAll('.tool-button').forEach(b => b.classList.remove('active'));
      this.classList.add('active');
      currentTool = this.dataset.tool;
   });
});

// Canvas event handlers
if (canvas) {
   canvas.addEventListener('mousedown', handleMouseDown);
   canvas.addEventListener('mousemove', handleMouseMove);
   canvas.addEventListener('mouseup', handleMouseUp);
   canvas.addEventListener('dblclick', handleDoubleClick);
}

function handleMouseDown(e) {
   const rect = canvas.getBoundingClientRect();
   const x = e.clientX - rect.left;
   const y = e.clientY - rect.top;
    
   if (currentTool === 'select') {
      // Check if clicking on existing shape
      for (let i = shapes.length - 1; i >= 0; i--) {
         if (shapes[i].contains(x, y)) {
            selectedShape = shapes[i];
            isDragging = true;
            dragOffsetX = x - selectedShape.x;
            dragOffsetY = y - selectedShape.y;
            updatePropertyEditor();
            redraw();
            return;
         }
      }
      selectedShape = null;
      updatePropertyEditor();
      redraw();
   } else if (currentTool === 'line' || currentTool === 'arrow') {
      // Check if clicking on a connection point
      for (let i = shapes.length - 1; i >= 0; i--) {
         const shape = shapes[i];
         if (shape.type !== 'line' && shape.type !== 'arrow' && shape.type !== 'text') {
            const point = shape.findConnectionPoint(x, y);
            if (point) {
               drawingConnection = true;
               startConnectionPoint = { shape: shape, point: point };
               const coords = shape.getConnectionCoordinates(point);
               startX = coords.x;
               startY = coords.y;
               return;
            }
         }
      }
      // If not on connection point, start regular drawing
      isDrawing = true;
      startX = x;
      startY = y;
   } else {
      isDrawing = true;
      startX = x;
      startY = y;
   }
}

function handleMouseMove(e) {
   const rect = canvas.getBoundingClientRect();
   const x = e.clientX - rect.left;
   const y = e.clientY - rect.top;
   
   // Update hovered connection point
   hoveredConnectionPoint = null;
   if (currentTool === 'line' || currentTool === 'arrow' || currentTool === 'select') {
      for (let shape of shapes) {
         if (shape.type !== 'line' && shape.type !== 'arrow' && shape.type !== 'text') {
            const point = shape.findConnectionPoint(x, y);
            if (point) {
               hoveredConnectionPoint = { shape: shape, point: point };
               canvas.style.cursor = 'crosshair';
               if (!isDragging && !isDrawing && !drawingConnection) {
                  redraw();
               }
               break;
            }
         }
      }
      if (!hoveredConnectionPoint && !isDragging && !isDrawing && !drawingConnection) {
         canvas.style.cursor = currentTool === 'select' ? 'default' : 'crosshair';
      }
   }
    
   if (isDragging && selectedShape) {
      if (selectedShape.type !== 'line' && selectedShape.type !== 'arrow') {
         selectedShape.x = x - dragOffsetX;
         selectedShape.y = y - dragOffsetY;
      }
      redraw();
   } else if (drawingConnection || isDrawing) {
      redraw();
      ctx.strokeStyle = '#666';
      ctx.lineWidth = 2;
      ctx.setLineDash([5, 5]);
        
      if (drawingConnection || currentTool === 'line' || currentTool === 'arrow') {
         ctx.beginPath();
         ctx.moveTo(startX, startY);
         ctx.lineTo(x, y);
         ctx.stroke();
      } else {
         const width = x - startX;
         const height = y - startY;
        
         switch(currentTool) {
            case 'circle':
               ctx.beginPath();
               const radius = Math.min(Math.abs(width), Math.abs(height)) / 2;
               ctx.arc(startX + width/2, startY + height/2, radius, 0, 2 * Math.PI);
               ctx.stroke();
               break;
            case 'rectangle':
               ctx.strokeRect(startX, startY, width, height);
               break;
         }
      }
      ctx.setLineDash([]);
   }
}

function handleMouseUp(e) {
   const rect = canvas.getBoundingClientRect();
   const x = e.clientX - rect.left;
   const y = e.clientY - rect.top;
   
   if (drawingConnection) {
      // Check if released on a connection point
      for (let shape of shapes) {
         if (shape.type !== 'line' && shape.type !== 'arrow' && shape.type !== 'text') {
            const point = shape.findConnectionPoint(x, y);
            if (point && shape !== startConnectionPoint.shape) {
               // Create connected line/arrow
               const connector = new Shape(currentTool, 0, 0, 0, 0);
               connector.startNode = startConnectionPoint.shape;
               connector.startPoint = startConnectionPoint.point;
               connector.endNode = shape;
               connector.endPoint = point;
               shapes.push(connector);
               drawingConnection = false;
               startConnectionPoint = null;
               updateDOTPreview();
               redraw();
               return;
            }
         }
      }
      // If not released on connection point, cancel
      drawingConnection = false;
      startConnectionPoint = null;
      redraw();
   } else if (isDrawing) {
      const width = x - startX;
      const height = y - startY;
        
      if (currentTool === 'text') {
         const text = prompt('Enter text:');
         if (text) {
            shapes.push(new Shape('text', startX, startY, 0, 0, text));
         }
      } else if (currentTool === 'line' || currentTool === 'arrow') {
         // Create unconnected line/arrow
         const connector = new Shape(currentTool, startX, startY, x - startX, y - startY);
         shapes.push(connector);
      } else if (Math.abs(width) > 5 && Math.abs(height) > 5) {
         shapes.push(new Shape(currentTool, 
            Math.min(startX, x), 
            Math.min(startY, y), 
            Math.abs(width), 
            Math.abs(height)));
      }
        
      isDrawing = false;
      updateDOTPreview();
      redraw();
   }
   isDragging = false;
}

function handleDoubleClick(e) {
    if (selectedShape && selectedShape.type !== 'text' && selectedShape.type !== 'line' && selectedShape.type !== 'arrow') {
        const text = prompt('Enter label:', selectedShape.text);
        if (text !== null) {
            selectedShape.text = text;
            redraw();
        }
    }
}

function redraw() {
   if (!ctx) return;
   ctx.clearRect(0, 0, canvas.width, canvas.height);
   shapes.forEach(shape => shape.draw());
}

function updatePropertyEditor() {
   const editorContent = document.getElementById('propertyEditorContent');
   if (!editorContent) return;
    
   if (!selectedShape) {
      editorContent.innerHTML = '<p class="text-muted"><small>Select a shape to edit its properties</small></p>';
      return;
   }
    
   let html = `
      <div class="mb-3">
         <label class="form-label"><strong>Shape Type:</strong> ${selectedShape.type}</label>
      </div>
   `;
    
   if (selectedShape.type !== 'line' && selectedShape.type !== 'arrow') {
      html += `
         <div class="mb-3">
            <label for="shapeLabel" class="form-label">Label</label>
            <input type="text" class="form-control form-control-sm" id="shapeLabel" value="${selectedShape.text || ''}" onchange="updateShapeProperty('text', this.value)">
         </div>
      `;
   }
    
   if (selectedShape.type !== 'text') {
      html += `
         <div class="mb-3">
            <label for="strokeColor" class="form-label">Border Color</label>
            <input type="color" class="form-control form-control-color" id="strokeColor" value="${selectedShape.strokeColor}" onchange="updateShapeProperty('strokeColor', this.value)">
         </div>
         <div class="mb-3">
            <label for="lineWidth" class="form-label">Border Width: <span id="lineWidthValue">${selectedShape.lineWidth}</span></label>
            <input type="range" class="form-range" id="lineWidth" min="1" max="10" value="${selectedShape.lineWidth}" oninput="document.getElementById('lineWidthValue').textContent=this.value" onchange="updateShapeProperty('lineWidth', parseInt(this.value))">
         </div>
      `;
   }
    
   if (selectedShape.type === 'circle' || selectedShape.type === 'rectangle') {
      html += `
         <div class="mb-3">
            <label for="fillColor" class="form-label">Fill Color</label>
            <input type="color" class="form-control form-control-color" id="fillColor" value="${selectedShape.fillColor}" onchange="updateShapeProperty('fillColor', this.value)">
         </div>
      `;
   }
    
   html += `
      <button class="btn btn-danger btn-sm w-100" onclick="deleteSelectedShape()">Delete Shape</button>
   `;
    
   editorContent.innerHTML = html;
}

function updateShapeProperty(property, value) {
   if (selectedShape) {
      selectedShape[property] = value;
      redraw();
      updateDOTPreview();
   }
}

function deleteSelectedShape() {
   if (selectedShape) {
      shapes = shapes.filter(s => s !== selectedShape);
      selectedShape = null;
      updatePropertyEditor();
      redraw();
      updateDOTPreview();
    }
}

function clearCanvas() {
   if (confirm('Are you sure you want to clear the canvas?')) {
      shapes = [];
      selectedShape = null;
      updatePropertyEditor();
      redraw();
      updateDOTPreview();
   }
}

function updateDOTPreview() {
    const previewDiv = document.getElementById('dotPreview');
    if (!previewDiv) return;
    
    let dot = 'digraph G {\n';
    dot += '  node [shape=record];\n';
    dot += '  rankdir=TB;\n\n';
    
    const nodes = shapes.filter(s => s.type === 'circle' || s.type === 'rectangle');
    const edges = shapes.filter(s => s.type === 'arrow' || s.type === 'line');
    
    // Export nodes
    nodes.forEach(shape => {
        const label = shape.text || `node${shape.id}`;
        const shapeType = shape.type === 'circle' ? 'ellipse' : 'box';
        const fillColor = shape.fillColor.replace('#', '');
        const strokeColor = shape.strokeColor.replace('#', '');
        dot += `  node${shape.id} [label="${label}", shape=${shapeType}, style=filled, fillcolor="#${fillColor}", color="#${strokeColor}", penwidth=${shape.lineWidth}];\n`;
    });
    
    dot += '\n';
    
    // Export edges
    edges.forEach(edge => {
        if (edge.startNode && edge.endNode) {
            const edgeOp = edge.type === 'arrow' ? '->' : '--';
            const strokeColor = edge.strokeColor.replace('#', '');
            dot += `  node${edge.startNode.id} ${edgeOp} node${edge.endNode.id} [color="#${strokeColor}", penwidth=${edge.lineWidth}];\n`;
        }
    });
    
    dot += '}\n';
    
    previewDiv.textContent = dot;
}

function exportToDOT() {
   let dot = 'digraph G {\n';
   dot += '  node [shape=record];\n';
   dot += '  rankdir=TB;\n\n';
      
   const nodes = shapes.filter(s => s.type === 'circle' || s.type === 'rectangle');
   const edges = shapes.filter(s => s.type === 'arrow' || s.type === 'line');
      
   // Export nodes
   nodes.forEach(shape => {
      const label = shape.text || `node${shape.id}`;
      const shapeType = shape.type === 'circle' ? 'ellipse' : 'box';
      const fillColor = shape.fillColor.replace('#', '');
      const strokeColor = shape.strokeColor.replace('#', '');
      dot += `  node${shape.id} [label="${label}", shape=${shapeType}, style=filled, fillcolor="#${fillColor}", color="#${strokeColor}", penwidth=${shape.lineWidth}];\n`;
   });
      
   dot += '\n';
      
   // Export edges
   edges.forEach(edge => {
      if (edge.startNode && edge.endNode) {
         const edgeOp = edge.type === 'arrow' ? '->' : '--';
         const strokeColor = edge.strokeColor.replace('#', '');
         dot += `  node${edge.startNode.id} ${edgeOp} node${edge.endNode.id} [color="#${strokeColor}", penwidth=${edge.lineWidth}];\n`;
      }
   });
      
   dot += '}\n';
      
   // Download DOT file
   const blob = new Blob([dot], { type: 'text/plain' });
   const url = URL.createObjectURL(blob);
   const a = document.createElement('a');
   a.href = url;
   a.download = 'diagram.dot';
   a.click();
   URL.revokeObjectURL(url);
      
   console.log('Exported DOT:\n', dot);
}

// Handle delete key
document.addEventListener('keydown', (e) => {
   if (e.key === 'Backspace' && selectedShape) {
      shapes = shapes.filter(s => s !== selectedShape);
      selectedShape = null;
      updatePropertyEditor();
      redraw();
   }
});