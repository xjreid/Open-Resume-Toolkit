import type { ResumeSection } from "@ort/contracts/resume";
import { createSection } from "./resume-editor";

export const STARTING_PROFILES = [
  {
    id: "general",
    label: "No preference",
    sections: ["Experience", "Education", "Skills"],
  },
  {
    id: "student",
    label: "Student/Recent Graduate",
    sections: [
      "Education",
      "Projects",
      "Internships",
      "Skills",
      "Activities and Organizations",
    ],
  },
  {
    id: "experienced",
    label: "Experienced Professional",
    sections: [
      "Professional Summary",
      "Experience",
      "Education",
      "Skills",
      "Certifications and Licenses",
    ],
  },
  {
    id: "technical",
    label: "Technical",
    sections: ["Experience", "Projects", "Skills", "Education"],
  },
  {
    id: "academic",
    label: "Academic/Research",
    sections: [
      "Education",
      "Research",
      "Publications",
      "Awards and Honors",
      "Experience",
    ],
  },
  {
    id: "sales",
    label: "Sales/Marketing",
    sections: [
      "Professional Summary",
      "Experience",
      "Accomplishments",
      "Skills",
      "Education",
    ],
  },
  { id: "custom", label: "Custom", sections: [] },
] as const;

export type StartingProfile = (typeof STARTING_PROFILES)[number]["id"];

export function createStartingSections(
  profile: StartingProfile,
): ResumeSection[] {
  const selected = STARTING_PROFILES.find(
    (candidate) => candidate.id === profile,
  )!;
  return selected.sections.map((heading, order) => ({
    ...createSection(order),
    heading,
  }));
}

export const SUGGESTED_SECTIONS = [
  "Professional Summary",
  "Education",
  "Experience",
  "Internships",
  "Projects",
  "Skills",
  "Certifications and Licenses",
  "Awards and Honors",
  "Leadership",
  "Volunteer Experience",
  "Research",
  "Publications",
  "Coursework",
  "Activities and Organizations",
  "Languages",
  "Accomplishments",
  "Portfolio and Professional Links",
  "Custom Section",
] as const;
