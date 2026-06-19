import type { ResumeCard, ResumeDossier, ResumeTarget } from "./types";

export function normalizeResumeCardForDossier(card: ResumeCard, dossier: ResumeDossier): ResumeCard {
  const warnings = [...(card.instrumentationWarnings ?? [])];
  let resumeTarget = card.resumeTarget;

  if (dossier.mode === "returned_to_origin") {
    if (dossier.candidateOriginAnchors.length === 0 && resumeTarget) {
      warnings.push("Model returned a resume target even though no origin anchors were captured.");
      resumeTarget = null;
    }

    if (resumeTarget && !sameLogicalUrl(resumeTarget.url, dossier.origin.url)) {
      warnings.push("Model returned a branch-page resume target; branch pages were kept as evidence only.");
      resumeTarget = null;
    }
  }

  return {
    ...card,
    branchFindings: normalizeStringArray(card.branchFindings),
    suggestedNextMessage: card.suggestedNextMessage || fallbackSuggestedNextMessage(card),
    instrumentationWarnings: Array.from(new Set([...warnings, ...dossier.instrumentationWarnings])).slice(0, 8),
    resumeTarget: resumeTarget ? sanitizeResumeTarget(resumeTarget) : null
  };
}

export function sameLogicalUrl(left: string | undefined, right: string | undefined): boolean {
  if (!left || !right) return false;
  try {
    const a = new URL(left);
    const b = new URL(right);
    return a.origin === b.origin && a.pathname === b.pathname && a.search === b.search;
  } catch {
    return left === right;
  }
}

function sanitizeResumeTarget(target: ResumeTarget): ResumeTarget {
  return Object.fromEntries(Object.entries(target).filter(([, value]) => value !== null)) as unknown as ResumeTarget;
}

function normalizeStringArray(value: string[] | undefined): string[] {
  return Array.isArray(value) ? value.filter((item) => typeof item === "string" && item.trim()).slice(0, 8) : [];
}

function fallbackSuggestedNextMessage(card: ResumeCard): string {
  if (card.branchFindings?.length) {
    return `Use these findings to continue the original task: ${card.branchFindings.slice(0, 3).join(" ")}`;
  }
  return card.summary || "Help me continue the original task from where I left off.";
}
