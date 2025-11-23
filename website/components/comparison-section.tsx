import { Check, Minus, X } from "lucide-react";

const frameworks = [
  {
    name: "Ultimo",
    features: {
      performance: "152k+",
      typeScript: "auto",
      openAPI: "auto",
      rpc: true,
      rest: true,
      validation: true,
      auth: true,
      clientGen: "auto",
      cors: true,
      rateLimit: true,
      devServer: true,
    },
  },
  {
    name: "Axum",
    features: {
      performance: "152k+",
      typeScript: "manual",
      openAPI: "manual",
      rpc: false,
      rest: true,
      validation: "manual",
      auth: "manual",
      clientGen: "manual",
      cors: "manual",
      rateLimit: "manual",
      devServer: false,
    },
  },
  {
    name: "Actix Web",
    features: {
      performance: "140k+",
      typeScript: "manual",
      openAPI: "manual",
      rpc: false,
      rest: true,
      validation: "manual",
      auth: "manual",
      clientGen: "manual",
      cors: "manual",
      rateLimit: "manual",
      devServer: false,
    },
  },
  {
    name: "Rocket",
    features: {
      performance: "130k+",
      typeScript: "manual",
      openAPI: "manual",
      rpc: false,
      rest: true,
      validation: true,
      auth: "manual",
      clientGen: "manual",
      cors: "manual",
      rateLimit: "manual",
      devServer: false,
    },
  },
  {
    name: "Warp",
    features: {
      performance: "145k+",
      typeScript: "manual",
      openAPI: "manual",
      rpc: false,
      rest: true,
      validation: "manual",
      auth: "manual",
      clientGen: "manual",
      cors: "manual",
      rateLimit: "manual",
      devServer: false,
    },
  },
];

const featureLabels = {
  performance: "Performance",
  typeScript: "TypeScript Types",
  openAPI: "OpenAPI Generation",
  rpc: "JSON-RPC Support",
  rest: "REST Support",
  validation: "Request Validation",
  auth: "Authentication",
  clientGen: "Client Generation",
  cors: "CORS",
  rateLimit: "Rate Limiting",
  devServer: "Dev Server",
};

function FeatureCell({ value }: { value: string | boolean }) {
  if (value === "auto") {
    return (
      <div className="flex items-center justify-center gap-2">
        <Check className="h-5 w-5 text-green-500" />
        <span className="text-sm text-green-500 font-medium">Auto</span>
      </div>
    );
  }
  if (value === "manual") {
    return (
      <div className="flex items-center justify-center gap-2">
        <Minus className="h-5 w-5 text-yellow-500" />
        <span className="text-sm text-yellow-500 font-medium">Manual</span>
      </div>
    );
  }
  if (value === true) {
    return (
      <div className="flex items-center justify-center">
        <Check className="h-5 w-5 text-green-500" />
      </div>
    );
  }
  if (value === false) {
    return (
      <div className="flex items-center justify-center">
        <X className="h-5 w-5 text-red-500/50" />
      </div>
    );
  }
  // Performance values
  return (
    <div className="flex items-center justify-center">
      <span className="text-sm font-medium text-green-500">{value} req/s</span>
    </div>
  );
}

export function ComparisonSection() {
  return (
    <section className="py-24 relative overflow-hidden">
      <div className="absolute inset-0 -z-10">
        <div className="absolute top-[20%] right-[10%] w-[600px] h-[600px] bg-orange-500/5 blur-[120px] rounded-full" />
      </div>

      <div className="container px-4 md:px-6 mx-auto">
        <div className="text-center max-w-3xl mx-auto mb-16">
          <h2 className="text-3xl md:text-5xl font-bold mb-6 tracking-tight">
            How Ultimo Compares
          </h2>
          <p className="text-muted-foreground text-lg">
            Ultimo builds on the best of Rust web frameworks while adding
            powerful full-stack features that save you time and reduce
            boilerplate.
          </p>
        </div>

        {/* Desktop Table */}
        <div className="hidden lg:block">
          <div className="rounded-xl border border-border bg-card/50 backdrop-blur-sm overflow-hidden">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left p-4 font-semibold text-foreground">
                      Feature
                    </th>
                    {frameworks.map((fw) => (
                      <th key={fw.name} className="text-center p-4">
                        <span
                          className={`font-semibold ${
                            fw.name === "Ultimo"
                              ? "text-orange-500"
                              : "text-foreground"
                          }`}
                        >
                          {fw.name}
                        </span>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {Object.entries(featureLabels).map(([key, label], index) => (
                    <tr
                      key={key}
                      className={`border-b border-border/50 hover:bg-accent/50 transition-colors ${
                        index % 2 === 0 ? "bg-muted/20" : ""
                      }`}
                    >
                      <td className="p-4 font-medium text-foreground">
                        {label}
                      </td>
                      {frameworks.map((fw) => (
                        <td
                          key={fw.name}
                          className={`p-4 ${
                            fw.name === "Ultimo" ? "bg-orange-500/5" : ""
                          }`}
                        >
                          <FeatureCell
                            value={fw.features[key as keyof typeof fw.features]}
                          />
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* Mobile Cards */}
        <div className="lg:hidden space-y-6">
          {frameworks.map((fw) => (
            <div
              key={fw.name}
              className={`rounded-xl border p-6 ${
                fw.name === "Ultimo"
                  ? "border-orange-500/50 bg-orange-500/5"
                  : "border-border bg-card/50"
              }`}
            >
              <div className="flex items-center gap-3 mb-6 pb-4 border-b border-border">
                <h3
                  className={`text-xl font-bold ${
                    fw.name === "Ultimo" ? "text-orange-500" : "text-foreground"
                  }`}
                >
                  {fw.name}
                </h3>
              </div>
              <div className="space-y-3">
                {Object.entries(featureLabels).map(([key, label]) => (
                  <div
                    key={key}
                    className="flex items-center justify-between py-2"
                  >
                    <span className="text-sm text-muted-foreground">
                      {label}
                    </span>
                    <FeatureCell
                      value={fw.features[key as keyof typeof fw.features]}
                    />
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>

        <div className="mt-12 text-center">
          <p className="text-sm text-muted-foreground">
            <Check className="inline h-4 w-4 text-green-500 mr-1" />
            Built-in &nbsp;&nbsp;
            <Minus className="inline h-4 w-4 text-yellow-500 mr-1" />
            Requires manual setup/crates &nbsp;&nbsp;
            <X className="inline h-4 w-4 text-red-500/50 mr-1" />
            Not available
          </p>
        </div>
      </div>
    </section>
  );
}
