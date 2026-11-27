// What kind of app this is, in plain words, and which tabs make sense for it.
// Derived only from what the analysis already found: the project kind, the recognized AI
// system, the routes, UI elements and data models. Nothing here guesses beyond that.

import type { Indexed } from "./graph";

export type AppShape =
  | "ai-fullstack"
  | "ai-service"
  | "ai-model"
  | "fullstack"
  | "server"
  | "frontend"
  | "desktop"
  | "cli"
  | "library"
  | "unknown";

export interface AppProfile {
  shape: AppShape;
  /** Short label for badges, e.g. "AI + web app". */
  label: string;
  /** One sentence: what the app is and what the tabs show about it. */
  blurb: string;
  ai: boolean;
  hasUi: boolean;
  hasServer: boolean;
  hasData: boolean;
  /** Name of the "How it works" tab, matched to the app. */
  insightsLabel: string;
}

export function appProfile(ix: Indexed): AppProfile {
  const kind = ix.g.summary.kind;
  const ai = ix.g.insights?.ai ?? undefined;
  const hasUi = ix.ui.size > 0;
  const hasServer = ix.routes.size > 0 || kind === "backend" || kind === "full-stack";
  const hasData = ix.models.size > 0;
  const base = { ai: !!ai, hasUi, hasServer, hasData };
  if (
    ai &&
    (ai.kind === "modelTraining" || ai.kind === "inference" || ai.kind === "modelServing")
  ) {
    return {
      ...base,
      shape: "ai-model",
      label: "AI model code",
      blurb:
        "Code that trains, runs or serves a model. How it works shows the model pipeline; System shows where it runs.",
      insightsLabel: "How the AI works",
    };
  }
  if (ai && (kind === "full-stack" || kind === "desktop" || kind === "frontend") && hasUi) {
    return {
      ...base,
      shape: "ai-fullstack",
      label: "AI + app with a screen",
      blurb:
        "An AI system with a user interface in front of it. Mixed project: How it works covers the AI part, System and Code map cover the rest.",
      insightsLabel: "How the AI works",
    };
  }
  if (ai) {
    return {
      ...base,
      shape: "ai-service",
      label: "AI service (server only)",
      blurb: "An AI system that runs on a server with no screen of its own.",
      insightsLabel: "How the AI works",
    };
  }
  const plain: Record<string, Pick<AppProfile, "shape" | "label" | "blurb">> = {
    "full-stack": {
      shape: "fullstack",
      label: "Full-stack app",
      blurb: "A screen that talks to a server, usually with a database behind it.",
    },
    backend: {
      shape: "server",
      label: "Server only",
      blurb: "A server or API with no screen of its own.",
    },
    worker: {
      shape: "server",
      label: "Background worker",
      blurb: "Code that runs jobs in the background, with no screen.",
    },
    frontend: {
      shape: "frontend",
      label: "Front end only",
      blurb: "A screen with no server code in this folder.",
    },
    desktop: {
      shape: "desktop",
      label: "Desktop app",
      blurb: "A window plus native code that runs on this computer.",
    },
    cli: {
      shape: "cli",
      label: "Command-line tool",
      blurb: "A program you run from a terminal.",
    },
    library: {
      shape: "library",
      label: "Library",
      blurb: "Reusable code that other programs import.",
    },
  };
  const p = plain[kind] ?? {
    shape: "unknown" as AppShape,
    label: "Unclassified",
    blurb: "Not enough recognizable code to say what kind of app this is.",
  };
  return { ...base, ...p, insightsLabel: "Code insights" };
}
