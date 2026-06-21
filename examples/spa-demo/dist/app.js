// Minimal SPA: fetch /api/hello and display the result.
// In a real app this would be a compiled bundle (Vite, webpack, etc.).
fetch("/api/hello")
  .then((r) => r.json())
  .then((data) => {
    document.getElementById("app").textContent = data.message;
  })
  .catch((err) => {
    document.getElementById("app").textContent = "Error: " + err.message;
  });
