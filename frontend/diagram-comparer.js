function buildStudentDiagram() {
  // Prefer the real diagram from the Diagram Creator tab
  if (typeof window.getCurrentDiagram === "function") {
    const diag = window.getCurrentDiagram();
    if (diag && Array.isArray(diag.classes) && Array.isArray(diag.relationships)) {
      return diag;
    }
  }
  return {
    classes: [],
    relationships: [],
  };
}


function renderMistakes(mistakes) {
  const list = document.getElementById("dc-mistake-list");
  const noMistakes = document.getElementById("dc-no-mistakes");

  // Clear previous
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

    li.appendChild(title);
    li.appendChild(msg);

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

  if (!btn || !sourceArea) {
    return;
  }

  btn.addEventListener("click", async () => {
    setError(null);
    setLoading(true);
    renderMistakes([]); // clear old mistakes

    const sourceCode = sourceArea.value || "";

    const studentDiagram = buildStudentDiagram();

    const body = {
      source_code: sourceCode,
      student_diagram: studentDiagram,
    };

    try {
      const res = await fetch("http://localhost:3000/api/compare-diagrams", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (!res.ok) {
        throw new Error("Server error: " + res.status);
      }

      const data = await res.json();
      renderMistakes(data.mistakes || []);
    } catch (err) {
      console.error(err);
      setError(err.message || "Failed to compare diagrams.");
    } finally {
      setLoading(false);
    }
  });
});
