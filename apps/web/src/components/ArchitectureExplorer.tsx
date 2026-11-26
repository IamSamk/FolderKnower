"use client";

import { ArrowRight, CheckCircle2, FileCode, Route } from "lucide-react";
import { useState } from "react";

interface FlowStep {
  tier: "frontend" | "api" | "backend" | "database";
  label: string;
  detail: string;
  file: string;
  codeSnippet: string;
}

const DEMO_FLOWS: { id: string; name: string; description: string; steps: FlowStep[] }[] = [
  {
    id: "create-todo",
    name: "Create a todo",
    description: "From the Add button to an INSERT on the todos table",
    steps: [
      {
        tier: "frontend",
        label: "Add Todo Button",
        detail: "User submits form in UI",
        file: "web/src/components/TodoList.tsx",
        codeSnippet: "api.createTodo({ title, completed: false })",
      },
      {
        tier: "api",
        label: "POST /api/todos",
        detail: "REST route parsed by router",
        file: "api/app/routes.py",
        codeSnippet: "@router.post('/todos', response_model=TodoOut)",
      },
      {
        tier: "backend",
        label: "Todo Service Handler",
        detail: "Validates payload with Pydantic",
        file: "api/app/services.py",
        codeSnippet: "db_todo = models.Todo(**todo.dict())",
      },
      {
        tier: "database",
        label: "todos Table",
        detail: "SQLAlchemy commits record to disk",
        file: "db/schema.sql",
        codeSnippet: "INSERT INTO todos (title, done) VALUES ($1, $2)",
      },
    ],
  },
  {
    id: "user-login",
    name: "Sign in",
    description: "From the sign-in form through the password check to the users table",
    steps: [
      {
        tier: "frontend",
        label: "Login Form",
        detail: "Submits credentials",
        file: "web/src/components/LoginForm.tsx",
        codeSnippet: "auth.login(username, password)",
      },
      {
        tier: "api",
        label: "POST /api/auth/token",
        detail: "OAuth2 password flow route",
        file: "api/app/auth.py",
        codeSnippet: "@router.post('/auth/token', response_model=TokenOut)",
      },
      {
        tier: "backend",
        label: "Verify Password Hash",
        detail: "Bcrypt verification & JWT sign",
        file: "api/app/security.py",
        codeSnippet: "verify_password(plain, hashed_pw)",
      },
      {
        tier: "database",
        label: "users Table",
        detail: "Selects user by email index",
        file: "api/app/models.py",
        codeSnippet: "SELECT * FROM users WHERE email = $1 LIMIT 1",
      },
    ],
  },
];

export function ArchitectureExplorer() {
  const [activeFlowIndex, setActiveFlowIndex] = useState<number>(0);
  const [activeStepIndex, setActiveStepIndex] = useState<number>(0);

  const activeFlow = DEMO_FLOWS[activeFlowIndex];
  const activeStep = activeFlow.steps[activeStepIndex];

  return (
    <section
      id="explorer"
      className="scroll-mt-16 py-24 border-t-[2.5px] border-[var(--line)] bg-[var(--surface-2)]"
    >
      <div className="max-w-6xl mx-auto px-6">
        {/* Section Header */}
        <div className="max-w-2xl mb-12">
          <p className="ak-eyebrow mb-3">How it works</p>
          <h2 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-[var(--ink)]">
            From a click to the database row
          </h2>
          <p className="text-[var(--ink-2)] text-base font-medium mt-3">
            FolderKnower reads the syntax tree of each file. It links the button to the API call,
            the route, the service and the table, and keeps the file and line for each step. Try a
            sample trace below.
          </p>
        </div>

        {/* Flow Selector Tabs */}
        <div
          className="flex flex-wrap items-center gap-3 mb-8"
          role="group"
          aria-label="Sample traces"
        >
          {DEMO_FLOWS.map((flow, idx) => (
            <button
              key={flow.id}
              type="button"
              aria-pressed={activeFlowIndex === idx}
              onClick={() => {
                setActiveFlowIndex(idx);
                setActiveStepIndex(0);
              }}
              className={`ak-btn text-sm px-4 py-2 ${
                activeFlowIndex === idx ? "ak-btn-accent" : ""
              }`}
            >
              <Route size={15} />
              <span>{flow.name}</span>
            </button>
          ))}
        </div>

        {/* Visual Request Flow Canvas */}
        <div className="ak-card p-6 sm:p-8 mb-8">
          <div className="text-xs font-bold uppercase tracking-wider text-[var(--ink-3)] mb-6 flex flex-wrap items-center justify-between gap-2">
            <span>{activeFlow.description}</span>
            <span className="text-[var(--ink-2)]">Click a step to see its code</span>
          </div>

          {/* Sequential 4-Tier Node Pipeline */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 relative">
            {activeFlow.steps.map((step, idx) => {
              const isSelected = activeStepIndex === idx;
              const tierColor =
                step.tier === "frontend"
                  ? "var(--tier-ui)"
                  : step.tier === "api"
                    ? "var(--tier-gateway)"
                    : step.tier === "backend"
                      ? "var(--tier-backend)"
                      : "var(--tier-storage)";

              const tierText =
                step.tier === "frontend"
                  ? "Frontend UI"
                  : step.tier === "api"
                    ? "API Gateway"
                    : step.tier === "backend"
                      ? "Service Logic"
                      : "Database Layer";

              return (
                <button
                  key={step.label}
                  type="button"
                  aria-pressed={isSelected}
                  onClick={() => setActiveStepIndex(idx)}
                  className={`ak-card text-left p-4 cursor-pointer relative ${
                    isSelected
                      ? "bg-[var(--surface-3)] shadow-[var(--sh)] -translate-x-0.5 -translate-y-0.5"
                      : "bg-[var(--surface-2)] shadow-[var(--sh-sm)] hover:-translate-x-0.5 hover:-translate-y-0.5 hover:shadow-[var(--sh)]"
                  }`}
                >
                  <div className="flex items-center justify-between mb-3">
                    <span
                      className="ak-badge text-[10px] uppercase font-black px-2 py-0.5 shadow-none"
                      style={{ backgroundColor: tierColor, color: "var(--on-tier)" }}
                    >
                      {tierText}
                    </span>
                    <span className="font-mono text-xs font-bold text-[var(--ink-3)]">
                      0{idx + 1}
                    </span>
                  </div>

                  <h3 className="font-extrabold text-sm text-[var(--ink)] mb-1">{step.label}</h3>

                  <p className="text-xs text-[var(--ink-2)] font-medium leading-relaxed">
                    {step.detail}
                  </p>

                  {idx < activeFlow.steps.length - 1 && (
                    <div className="hidden lg:block absolute -right-3.5 top-1/2 -translate-y-1/2 z-20">
                      <div className="w-6 h-6 rounded-full bg-[var(--accent)] text-[var(--on-accent)] border-2 border-[var(--line)] flex items-center justify-center">
                        <ArrowRight size={12} strokeWidth={3} />
                      </div>
                    </div>
                  )}
                </button>
              );
            })}
          </div>
        </div>

        {/* Step Inspector & Static Code Evidence */}
        <div className="ak-card p-6 sm:p-8 bg-[var(--code-bg)]" aria-live="polite">
          <div className="flex flex-col md:flex-row md:items-center justify-between pb-4 mb-4 border-b-2 border-[var(--hair)] gap-4">
            <div className="flex items-center gap-3">
              <FileCode size={22} strokeWidth={2.25} className="text-[var(--ink-2)] shrink-0" />
              <div>
                <span className="text-[11px] font-bold text-[var(--ink-3)] uppercase tracking-wider block">
                  Evidence
                </span>
                <span className="font-mono font-bold text-sm text-[var(--ink)]">
                  {activeStep.file}
                </span>
              </div>
            </div>

            <div className="flex items-center gap-2 text-sm font-bold text-[var(--ok)]">
              <CheckCircle2 size={16} strokeWidth={2.5} />
              <span>Linked from the syntax tree</span>
            </div>
          </div>

          <div className="bg-[var(--surface)] p-4 rounded-[var(--r-sm)] border-2 border-[var(--hair)] font-mono text-sm text-[var(--ink)] overflow-x-auto whitespace-nowrap">
            <span className="text-[var(--ink-3)] select-none mr-3">1</span>
            <span className="font-bold">{activeStep.codeSnippet}</span>
          </div>
        </div>
      </div>
    </section>
  );
}
