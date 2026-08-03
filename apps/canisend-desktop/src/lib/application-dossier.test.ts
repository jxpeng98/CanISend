import { describe, expect, it } from "vitest";

import type { ApplicationDossierReadModel } from "./bridge";
import { daysUntilDeadline, upcomingDeadlineApplications } from "./application-dossier";

function dossier(
  id: string,
  deadline: string | null,
  state: ApplicationDossierReadModel["state"] = "in-progress",
): ApplicationDossierReadModel {
  return {
    workspace: "/tmp/workspace",
    job: {
      id,
      title: `Application ${id}`,
      institution: "University",
      source_ids: [],
      created_at: "2026-07-30T00:00:00Z",
      revision: 1,
      archived: state === "archived",
    },
    metadata: {
      origin: "direct",
      discovery_lead_id: null,
      discovery_source_id: null,
      location: null,
      deadline,
      source_url: null,
      freshness: null,
      last_seen_at: null,
    },
    source_count: 0,
    profile_source_count: 0,
    state,
    current_stage: "intake",
    completed_stages: 0,
    total_stages: 10,
    workflow: null,
    blockers: [],
    next_actions: [],
  };
}

describe("application dossier deadlines", () => {
  const today = new Date(2026, 6, 30, 12);

  it("calculates calendar-day distance without UTC date drift", () => {
    expect(daysUntilDeadline("2026-07-30", today)).toBe(0);
    expect(daysUntilDeadline("2026-08-01", today)).toBe(2);
    expect(daysUntilDeadline("2026-02-30", today)).toBeNull();
    expect(daysUntilDeadline(null, today)).toBeNull();
  });

  it("returns active deadlines in urgency order", () => {
    const result = upcomingDeadlineApplications(
      [
        dossier("later", "2026-08-20"),
        dossier("soon", "2026-08-01"),
        dossier("archived", "2026-07-31", "archived"),
        dossier("unknown", null),
      ],
      30,
      today,
    );

    expect(result.map((application) => application.job.id)).toEqual(["soon", "later"]);
  });
});
