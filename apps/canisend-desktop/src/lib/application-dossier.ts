import type { ApplicationDossierReadModel } from "$lib/bridge";

const DAY_MILLIS = 24 * 60 * 60 * 1_000;

export function daysUntilDeadline(
  deadline: string | null,
  today = new Date(),
): number | null {
  if (!deadline || !/^\d{4}-\d{2}-\d{2}$/.test(deadline)) return null;
  const [year, month, day] = deadline.split("-").map(Number);
  if (year === undefined || month === undefined || day === undefined) return null;
  const deadlineUtc = Date.UTC(year, month - 1, day);
  const parsed = new Date(deadlineUtc);
  if (
    parsed.getUTCFullYear() !== year ||
    parsed.getUTCMonth() !== month - 1 ||
    parsed.getUTCDate() !== day
  ) {
    return null;
  }
  const todayUtc = Date.UTC(
    today.getFullYear(),
    today.getMonth(),
    today.getDate(),
  );
  return Math.round((deadlineUtc - todayUtc) / DAY_MILLIS);
}

export function upcomingDeadlineApplications(
  applications: ApplicationDossierReadModel[],
  horizonDays = 30,
  today = new Date(),
): ApplicationDossierReadModel[] {
  return applications
    .filter((application) => application.state !== "archived")
    .map((application) => ({
      application,
      days: daysUntilDeadline(application.metadata.deadline, today),
    }))
    .filter(
      (
        entry,
      ): entry is {
        application: ApplicationDossierReadModel;
        days: number;
      } => entry.days !== null && entry.days >= 0 && entry.days <= horizonDays,
    )
    .sort(
      (left, right) =>
        left.days - right.days ||
        left.application.job.title.localeCompare(right.application.job.title),
    )
    .map((entry) => entry.application);
}
