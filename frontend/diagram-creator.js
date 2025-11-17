// Diagram Creator Logic
const canvas = document.getElementById('diagramCanvas');
const ctx = canvas ? canvas.getContext('2d') : null;

let currentTool = 'select';
let shapes = [];
let selectedShape = null;
let isDragging = false;
let isDrawing = false;
let startX, startY;
let dragOffsetX, dragOffsetY;
let nextShapeId = 1;

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
      this.startX = x;
      this.startY = y;
      this.endX = x + width;
      this.endY = y + height;
      this.strokeColor = '#000000';
      this.fillColor = '#ffffff';
      this.lineWidth = 2;
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
            ctx.beginPath();
            ctx.moveTo(this.startX, this.startY);
            ctx.lineTo(this.endX, this.endY);
            ctx.stroke();
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
   }
   
   drawArrow() {
      const headLength = 15;
      const angle = Math.atan2(this.endY - this.startY, this.endX - this.startX);
         
      ctx.beginPath();
      ctx.moveTo(this.startX, this.startY);
      ctx.lineTo(this.endX, this.endY);
      ctx.stroke();
         
      ctx.beginPath();
      ctx.moveTo(this.endX, this.endY);
      ctx.lineTo(this.endX - headLength * Math.cos(angle - Math.PI / 6),
                  this.endY - headLength * Math.sin(angle - Math.PI / 6));
      ctx.lineTo(this.endX - headLength * Math.cos(angle + Math.PI / 6),
                  this.endY - headLength * Math.sin(angle + Math.PI / 6));
      ctx.closePath();
      ctx.fillStyle = this.strokeColor;
      ctx.fill();
      }
    
   contains(x, y) {
      if (this.type === 'line' || this.type === 'arrow') {
         const distance = this.pointToLineDistance(x, y);
         return distance < 5;
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
    
   pointToLineDistance(px, py) {
      const dx = this.endX - this.startX;
      const dy = this.endY - this.startY;
      const length = Math.sqrt(dx*dx + dy*dy);
      if (length === 0) return Math.sqrt((px-this.startX)**2 + (py-this.startY)**2);
        
      const t = Math.max(0, Math.min(1, ((px - this.startX) * dx + (py - this.startY) * dy) / (length * length)));
      const projX = this.startX + t * dx;
      const projY = this.startY + t * dy;
      return Math.sqrt((px - projX)**2 + (py - projY)**2);
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
            if (selectedShape.type === 'line' || selectedShape.type === 'arrow') {
               dragOffsetX = x - selectedShape.startX;
               dragOffsetY = y - selectedShape.startY;
            }
            redraw();
            return;
         }
      }
      selectedShape = null;
      redraw();
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
    
   if (isDragging && selectedShape) {
      if (selectedShape.type === 'line' || selectedShape.type === 'arrow') {
         const dx = x - dragOffsetX - selectedShape.startX;
         const dy = y - dragOffsetY - selectedShape.startY;
         selectedShape.startX += dx;
         selectedShape.startY += dy;
         selectedShape.endX += dx;
         selectedShape.endY += dy;
      } else {
         selectedShape.x = x - dragOffsetX;
         selectedShape.y = y - dragOffsetY;
      }
      redraw();
   } else if (isDrawing) {
      redraw();
      ctx.strokeStyle = '#666';
      ctx.setLineDash([5, 5]);
        
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
         case 'line':
         case 'arrow':
            ctx.beginPath();
            ctx.moveTo(startX, startY);
            ctx.lineTo(x, y);
            ctx.stroke();
            break;
      }
      ctx.setLineDash([]);
   }
}

function handleMouseUp(e) {
   if (isDrawing) {
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
        
      const width = x - startX;
      const height = y - startY;
        
      if (currentTool === 'text') {
         const text = prompt('Enter text:');
         if (text) {
            shapes.push(new Shape('text', startX, startY, 0, 0, text));
         }
      } else if (currentTool === 'line' || currentTool === 'arrow') {
         const shape = new Shape(currentTool, 0, 0, 0, 0);
         shape.startX = startX;
         shape.startY = startY;
         shape.endX = x;
         shape.endY = y;
         shapes.push(shape);
      } else if (Math.abs(width) > 5 && Math.abs(height) > 5) {
         shapes.push(new Shape(currentTool, 
            Math.min(startX, x), 
            Math.min(startY, y), 
            Math.abs(width), 
            Math.abs(height)));
      }
        
      isDrawing = false;
      redraw();
   }
   isDragging = false;
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
   }
}

function deleteSelectedShape() {
   if (selectedShape) {
      shapes = shapes.filter(s => s !== selectedShape);
      selectedShape = null;
      updatePropertyEditor();
      redraw();
    }
}

function clearCanvas() {
   if (confirm('Are you sure you want to clear the canvas?')) {
      shapes = [];
      selectedShape = null;
      updatePropertyEditor();
      redraw();
   }
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