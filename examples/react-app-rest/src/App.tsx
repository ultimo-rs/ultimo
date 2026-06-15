import { RestExample } from "./pages/RestExample";

function App() {
  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-100 via-white to-slate-100">
      <header className="border-b bg-background/90 backdrop-blur">
        <div className="mx-auto flex w-full max-w-5xl flex-col gap-2 px-4 py-6 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-primary">
              Ultimo Examples
            </p>
            <h1 className="mt-1 text-2xl font-bold tracking-tight sm:text-3xl">
              REST API Showcase
            </h1>
          </div>
          <p className="max-w-md text-sm text-muted-foreground">
            Same shadcn UI language across examples with CRUD flows and
            highlighted request snippets.
          </p>
        </div>
      </header>

      <main className="mx-auto w-full max-w-5xl px-4 py-8">
        <RestExample />
      </main>
    </div>
  );
}

export default App;
