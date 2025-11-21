import { RestExample } from "./pages/RestExample";

function App() {
  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white border-b">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <h1 className="text-2xl font-bold text-blue-600">
              ⚡ Ultimo Framework - REST API Example
            </h1>
          </div>
        </div>
      </header>

      <main className="container mx-auto px-4 py-8">
        <RestExample />
      </main>
    </div>
  );
}

export default App;
