function diagramFromCode(source) {
  const classes = {};
  const lines   = source.split('\n');
 
  let currentClass = null;
  let braceDepth   = 0;
  let classDepth   = {};
 
  for (let raw of lines) {
    const line = raw.trim();
     const classMatch = /\b(abstract\s+)?(?:(interface)|class)\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(line);
    if (classMatch) {
      const isAbstract  = !!classMatch[1];
      const isInterface = !!classMatch[2];
      const name        = classMatch[3];
 
      const extendsMatch = /\bextends\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(line);
      const implMatch    = /\bimplements\s+([A-Za-z_][A-Za-z0-9_,\s]*)/.exec(line);
 
      classes[name] = {
        isInterface,
        isAbstract,
        fields:          [],
        methods:         [],
        extendsName:     extendsMatch ? extendsMatch[1] : null,
        implementsNames: implMatch
          ? implMatch[1].split(',').map(s => s.trim()).filter(Boolean)
          : [],
      };
 
      const openOnLine  = (line.match(/{/g) || []).length;
      const closeOnLine = (line.match(/}/g) || []).length;
      braceDepth += openOnLine - closeOnLine;
      classDepth[name] = braceDepth;
      currentClass = name;
      continue;
    }
 
    const opens  = (line.match(/{/g) || []).length;
    const closes = (line.match(/}/g) || []).length;
    braceDepth  += opens - closes;
 
    if (currentClass && braceDepth < classDepth[currentClass]) {
      currentClass = null;
      for (const [cn, cd] of Object.entries(classDepth)) {
        if (braceDepth >= cd) currentClass = cn;
      }
    }
 
    if (!currentClass) continue;

    if (typeof methodDepth === 'undefined') var methodDepth = 0;
    if (methodDepth > 0) {
      methodDepth += opens - closes;
      if (methodDepth < 0) methodDepth = 0;
      continue;
    }
    const fieldMatch = /^\s*(public|private|protected)?\s*(static\s+)?(final\s+)?([A-Za-z_][A-Za-z0-9_<>\[\]]*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*[=;]/.exec(line);
    if (fieldMatch && !line.includes('(')) {
      const keywords = new Set(['if','else','while','for','return','new','try','catch','switch','case','throw','import','package','void','int','boolean']);
      const fieldType = fieldMatch[4];
      const fieldName = fieldMatch[5];
      if (!keywords.has(fieldName) && !keywords.has(fieldType)) {
        classes[currentClass].fields.push({
          name:       fieldName,
          visibility: fieldMatch[1] || 'package',
          fieldType,
        });
      }
      continue;
    }
 
    const methodMatch = /^\s*(public|private|protected)?\s*(static\s+)?(abstract\s+)?([A-Za-z_][A-Za-z0-9_<>\[\]]*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/.exec(line);
    if (methodMatch) {
      const methodName = methodMatch[5];
      const keywords   = new Set(['if','else','while','for','switch','catch','synchronized']);
      if (methodName !== currentClass && !keywords.has(methodName)) {
        classes[currentClass].methods.push({
          name:       methodName,
          visibility: methodMatch[1] || 'package',
        });
        if (line.includes('{')) methodDepth = 1;
      }
    }
  }
 
  // extends + implements
  const relationships = [];
  for (const [name, info] of Object.entries(classes)) {
    if (info.extendsName && classes[info.extendsName]) {
      relationships.push({ from: name, to: info.extendsName, kind: 'Extends' });
    }
    for (const iface of info.implementsNames) {
      if (classes[iface]) {
        relationships.push({ from: name, to: iface, kind: 'Implements' });
      }
    }
  }
 
  return {
    classes: Object.entries(classes).map(([name, info]) => ({
      name,
      isInterface: info.isInterface,
      isAbstract:  info.isAbstract,
      fields:      info.fields,
      methods:     info.methods,
    })),
    relationships,
  };
}
 
function analyzeMistakes(correct, student) {
  const mistakes = [];
 
  const correctMap = new Map(correct.classes.map(c => [c.name, c]));
  const studentMap = new Map(student.classes.map(c => [c.name, c]));
 
  // Missing + extra classes 
  for (const [name, info] of correctMap) {
    if (!studentMap.has(name)) {
      const kind = info.isInterface ? 'interface' : 'class';
      mistakes.push({
        kind:             'Missing' + (info.isInterface ? 'Interface' : 'Class'),
        message:          `Your diagram is missing the ${kind} '${name}'.`,
        hint:             `The Java code defines a ${kind} called '${name}'. Add a ${info.isInterface ? 'Interface' : 'Class'} shape and name it '${name}'.`,
        related_elements: [name],
      });
    }
  }
 
  for (const [name] of studentMap) {
    if (!correctMap.has(name)) {
      mistakes.push({
        kind:             'ExtraClass',
        message:          `Your diagram has '${name}' which doesn't exist in the code.`,
        hint:             `There is no class or interface called '${name}' in the Java code. Remove it or check for a spelling mistake.`,
        related_elements: [name],
      });
    }
  }
 
  for (const [name, correctInfo] of correctMap) {
    const studentInfo = studentMap.get(name);
    if (!studentInfo) continue;
    if (correctInfo.isInterface && !studentInfo.isInterface) {
      mistakes.push({
        kind:             'WrongNodeType',
        message:          `'${name}' should be drawn as an Interface, not a Class.`,
        hint:             `In the code '${name}' is declared with 'interface'. Use the Interface shape (dashed border) instead.`,
        related_elements: [name],
      });
    } else if (!correctInfo.isInterface && studentInfo.isInterface) {
      mistakes.push({
        kind:             'WrongNodeType',
        message:          `'${name}' should be drawn as a Class, not an Interface.`,
        hint:             `In the code '${name}' is declared with 'class'. Use the Class shape instead.`,
        related_elements: [name],
      });
    }
  }
 
  for (const correctClass of correct.classes) {
    const studentClass = studentMap.get(correctClass.name);
    if (!studentClass) continue;
 
    const studentMethodMap = new Map(studentClass.methods.map(m => [m.name, m]));
    const correctMethodMap = new Map(correctClass.methods.map(m => [m.name, m]));
 
    for (const [mName, mInfo] of correctMethodMap) {
      if (!studentMethodMap.has(mName)) {
        mistakes.push({
          kind:             'MissingMethod',
          message:          `Method '${mName}' is missing from '${correctClass.name}'.`,
          hint:             `The code defines '${mInfo.visibility} ${mName}(...)' in '${correctClass.name}'. Add a ${_visToShapeName(mInfo.visibility)} shape inside '${correctClass.name}' named '${mName}'.`,
          related_elements: [correctClass.name, mName],
        });
      }
    }
 
    for (const [mName] of studentMethodMap) {
      if (!correctMethodMap.has(mName)) {
        mistakes.push({
          kind:             'ExtraMethod',
          message:          `'${correctClass.name}' has method '${mName}' which isn't in the code.`,
          hint:             `The Java code does not define '${mName}' inside '${correctClass.name}'. Remove it or check the spelling.`,
          related_elements: [correctClass.name, mName],
        });
      }
    }
 
    for (const [mName, correctM] of correctMethodMap) {
      const studentM = studentMethodMap.get(mName);
      if (!studentM || !studentM.visibility) continue;
      if (correctM.visibility !== studentM.visibility) {
        mistakes.push({
          kind:             'WrongMethodVisibility',
          message:          `Method '${mName}' in '${correctClass.name}' has the wrong visibility.`,
          hint:             `The code declares '${mName}' as '${correctM.visibility}'. Use a ${_visToShapeName(correctM.visibility)} shape instead of ${_visToShapeName(studentM.visibility)}.`,
          related_elements: [correctClass.name, mName],
        });
      }
    }
  }
 
  // Fields — missing, extra
  for (const correctClass of correct.classes) {
    const studentClass = studentMap.get(correctClass.name);
    if (!studentClass) continue;
 
    const studentFieldMap = new Map((studentClass.fields || []).map(f => [f.name, f]));
    const correctFieldMap = new Map(correctClass.fields.map(f => [f.name, f]));
 
    for (const [fName, fInfo] of correctFieldMap) {
      if (!studentFieldMap.has(fName)) {
        mistakes.push({
          kind:             'MissingField',
          message:          `Field '${fName}' is missing from '${correctClass.name}'.`,
          hint:             `The code defines '${fInfo.visibility} ${fInfo.fieldType} ${fName}' in '${correctClass.name}'. Add a Field shape inside '${correctClass.name}' named '${fName}'.`,
          related_elements: [correctClass.name, fName],
        });
      }
    }
 
    for (const [fName] of studentFieldMap) {
      if (!correctFieldMap.has(fName)) {
        mistakes.push({
          kind:             'ExtraField',
          message:          `'${correctClass.name}' has field '${fName}' which isn't in the code.`,
          hint:             `The Java code does not define a field called '${fName}' inside '${correctClass.name}'. Remove it or check the spelling.`,
          related_elements: [correctClass.name, fName],
        });
      }
    }
  }
  
  const correctRelMap = new Map(correct.relationships.map(r => [`${r.from}->${r.to}:${r.kind}`, r]));
  const studentRelMap = new Map((student.relationships || []).map(r => [`${r.from}->${r.to}:${r.kind}`, r]));
  const correctPairs  = new Map(correct.relationships.map(r => [`${r.from}->${r.to}`, r.kind]));
  const studentPairs  = new Map((student.relationships || []).map(r => [`${r.from}->${r.to}`, r.kind]));
 
  for (const [key, rel] of correctRelMap) {
    const pairKey = `${rel.from}->${rel.to}`;
    if (!studentRelMap.has(key)) {
      if (studentPairs.has(pairKey)) {
        mistakes.push({
          kind:             'WrongRelationshipType',
          message:          `The arrow from '${rel.from}' to '${rel.to}' should be '${rel.kind}', not '${studentPairs.get(pairKey)}'.`,
          hint:             `${_relHint(rel.kind)} Replace the current arrow with a ${rel.kind} arrow.`,
          related_elements: [rel.from, rel.to],
        });
      } else {
        mistakes.push({
          kind:             'MissingRelationship',
          message:          `Missing a '${rel.kind}' arrow from '${rel.from}' to '${rel.to}'.`,
          hint:             `${_relHint(rel.kind)} Connect '${rel.from}' to '${rel.to}' using the ${rel.kind} tool.`,
          related_elements: [rel.from, rel.to],
        });
      }
    }
  }
 
  for (const [key, rel] of studentRelMap) {
    const pairKey = `${rel.from}->${rel.to}`;
    if (!correctRelMap.has(key) && !correctPairs.has(pairKey)) {
      mistakes.push({
        kind:             'ExtraRelationship',
        message:          `Unexpected '${rel.kind}' arrow from '${rel.from}' to '${rel.to}'.`,
        hint:             `The code does not show a ${rel.kind} relationship between '${rel.from}' and '${rel.to}'. Remove this arrow.`,
        related_elements: [rel.from, rel.to],
      });
    }
  }
 
  return mistakes;
}
 
function _visToShapeName(visibility) {
  if (visibility === 'public')    return 'Public Method';
  if (visibility === 'private')   return 'Private Method';
  if (visibility === 'protected') return 'Protected Method';
  return 'Method';
}
 
function _relHint(kind) {
  if (kind === 'Extends')    return 'Extends uses a solid line with a hollow arrowhead.';
  if (kind === 'Implements') return 'Implements uses a dashed line with a hollow arrowhead.';
  if (kind === 'Calls')      return 'Method Call uses a solid blue line with a filled arrowhead.';
  return '';
}

function buildStudentDiagram() {
  if (typeof window.getCurrentDiagram === "function") {
    const diag = window.getCurrentDiagram();
    if (diag && Array.isArray(diag.classes) && Array.isArray(diag.relationships)) {
      return diag;
    }
  }
  return { classes: [], relationships: [] };
}

function renderMistakes(mistakes) {
  const list = document.getElementById("dc-mistake-list");
  const noMistakes = document.getElementById("dc-no-mistakes");

  list.innerHTML = "";

  if (!mistakes || mistakes.length === 0) {
    noMistakes.style.display = "block";
    noMistakes.innerHTML = "No mistakes found! Your diagram matches the code exactly.";
    noMistakes.style.color = "#198754";
    noMistakes.style.fontWeight = "bold";
    noMistakes.style.fontSize = "14px";
    return;
  }

  noMistakes.style.display = "none";

  mistakes.forEach((m) => {
    const li = document.createElement("li");
    li.style.marginBottom = "6px";

    const title = document.createElement("div");
    title.style.fontWeight = "bold";
    title.style.fontSize = "13px";
    title.textContent = m.kind;

    const msg = document.createElement("div");
    msg.style.fontSize = "13px";
    msg.textContent = m.message;

    const hint = document.createElement("div");
    hint.style.fontSize = "12px";
    hint.style.color = "#1a6fc4";
    hint.style.marginTop = "4px";
    hint.textContent = m.hint;

    li.appendChild(title);
    li.appendChild(msg);
    li.appendChild(hint);

    if (m.related_elements && m.related_elements.length > 0) {
      const rel = document.createElement("div");
      rel.style.fontSize = "12px";
      rel.style.color = "#555";
      rel.textContent = "Related: " + m.related_elements.join(", ");
      li.appendChild(rel);
    }

    list.appendChild(li);
  });
}

function setError(message) {
  const errorDiv = document.getElementById("dc-error");
  if (!errorDiv) return;
  if (message) {
    errorDiv.textContent = message;
    errorDiv.style.display = "block";
  } else {
    errorDiv.textContent = "";
    errorDiv.style.display = "none";
  }
}

function setLoading(isLoading) {
  const btn = document.getElementById("dc-check-button");
  if (!btn) return;
  btn.disabled = isLoading;
  btn.textContent = isLoading ? "Checking..." : "Check Diagram";
}

document.addEventListener("DOMContentLoaded", () => {
  const btn = document.getElementById("dc-check-button");
  const sourceArea = document.getElementById("dc-source-code");
 const label = document.getElementById("dc-diagram-label");
  if (label) {
    const diag = typeof window.getCurrentDiagram === "function"
      ? window.getCurrentDiagram()
      : null;
    const count = diag ? diag.classes.length : 0;
    label.textContent = count > 0
      ? `Comparing against: creator diagram (${count} class${count > 1 ? "es" : ""})`
      : "Comparing against: current creator diagram";
  }
  if (!btn || !sourceArea) return;

  btn.addEventListener("click", () => {
    setError(null);
    setLoading(true);
    renderMistakes([]);

    try {
      const sourceCode = sourceArea.value || "";
      const studentDiagram = buildStudentDiagram();
      const correctDiagram = diagramFromCode(sourceCode);
      const mistakes = analyzeMistakes(correctDiagram, studentDiagram);
      console.log("correct:", JSON.stringify(correctDiagram, null, 2));
      console.log("student:", JSON.stringify(studentDiagram, null, 2));
      console.log("mistakes:", JSON.stringify(mistakes, null, 2));      
      renderMistakes(mistakes);
    } catch (err) {
      console.error(err);
      setError(err.message || "Failed to compare diagrams.");
    } finally {
      setLoading(false);
    }
  });
});