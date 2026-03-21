function diagramFromCode(source) {
  const classes = {};
  let currentClass = null;

  for (const line of source.split('\n')) {
    const classMatch = /\b(?:abstract\s+)?class\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(line);
    if (classMatch) {
      currentClass = classMatch[1];
      if (!classes[currentClass]) classes[currentClass] = [];
      continue;
    }

    if (currentClass) {
      const methodMatch = /\b(?:public|private|protected)?\s*(?:static\s+)?[A-Za-z0-9_<>\[\]]+\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/.exec(line);
      if (methodMatch) {
        const methodName = methodMatch[1];
        if (methodName !== currentClass) {
          classes[currentClass].push(methodName);
        }
      }
    }
  }

  return {
    classes: Object.entries(classes).map(([name, methods]) => ({
      name,
      methods: methods.map(m => ({ name: m }))
    })),
    relationships: []
  };
}

function analyzeMistakes(correct, student) {
  const mistakes = [];

  const correctNames = new Set(correct.classes.map(c => c.name));
  const studentNames = new Set(student.classes.map(c => c.name));

  for (const name of correctNames) {
    if (!studentNames.has(name)) {
      mistakes.push({
        kind: 'MissingClass',
        message: `Your diagram is missing the class '${name}'.`,
        hint: `The Java code defines a class called '${name}'. Every class in the code needs its own box in your diagram. Try adding a Class shape and naming it '${name}'.`,
        related_elements: [name]
      });
    }
  }

  for (const name of studentNames) {
    if (!correctNames.has(name)) {
      mistakes.push({
        kind: 'ExtraClass',
        message: `Your diagram has a class '${name}' that doesn't exist in the code.`,
        hint: `Check the Java code — there is no class called '${name}' defined there. Remove this box from your diagram, or check if you misspelled the class name.`,
        related_elements: [name]
      });
    }
  }

  const correctRels = new Map(correct.relationships.map(r => [`${r.from}->${r.to}`, r]));
  const studentRels = new Map(student.relationships.map(r => [`${r.from}->${r.to}`, r]));

  for (const [key, rel] of correctRels) {
    if (!studentRels.has(key)) {
      mistakes.push({
        kind: 'MissingRelationship',
        message: `Your diagram is missing a relationship between '${rel.from}' and '${rel.to}'.`,
        hint: `The code shows that '${rel.from}' has a connection to '${rel.to}'. Add the correct arrow between these two classes.`,
        related_elements: [rel.from, rel.to]
      });
    }
  }

  for (const [key, rel] of studentRels) {
    if (!correctRels.has(key)) {
      mistakes.push({
        kind: 'ExtraRelationship',
        message: `Your diagram has an unexpected arrow from '${rel.from}' to '${rel.to}'.`,
        hint: `The code does not show any relationship between '${rel.from}' and '${rel.to}'. Check if you accidentally connected the wrong shapes, or if '${rel.to}' is actually a method or field rather than a class.`,
        related_elements: [rel.from, rel.to]
      });
    }
  }

  for (const correctClass of correct.classes) {
    const studentClass = student.classes.find(c => c.name === correctClass.name);
    const studentMethods = new Set(studentClass ? studentClass.methods.map(m => m.name) : []);

    for (const method of correctClass.methods) {
      if (!studentMethods.has(method.name)) {
        mistakes.push({
          kind: 'MissingMethod',
          message: `The method '${method.name}' is missing from your '${correctClass.name}' class box.`,
          hint: `The Java code defines a method called '${method.name}' inside the '${correctClass.name}' class. Add a Method shape inside '${correctClass.name}' and name it '${method.name}'.`,
          related_elements: [correctClass.name, method.name]
        });
      }
    }
  }

  return mistakes;
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
      renderMistakes(mistakes);
    } catch (err) {
      console.error(err);
      setError(err.message || "Failed to compare diagrams.");
    } finally {
      setLoading(false);
    }
  });
});