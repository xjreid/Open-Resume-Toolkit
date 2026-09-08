// Presentation hints only. Section names never change stored entry meaning.
interface EntryGuidance {
  heading: string;
  subheading: string;
  headingExample: string;
  subheadingExample: string;
  details: readonly string[];
}

const general: EntryGuidance = {
  heading: "Role or qualification",
  subheading: "Organization",
  headingExample: "e.g. Software Engineer",
  subheadingExample: "e.g. Company or organization",
  details: ["Subtitle", "Technologies", "Metrics", "Custom detail"],
};

const guidance: Record<string, EntryGuidance> = {
  education: {
    heading: "Institution or qualification",
    subheading: "Degree or field of study",
    headingExample: "e.g. University name",
    subheadingExample: "e.g. Bachelor of Science, Computer Science",
    details: [
      "Degree",
      "Field of study",
      "GPA",
      "Honors",
      "Coursework",
      "Activities",
    ],
  },
  projects: {
    heading: "Project name",
    subheading: "Role or subtitle",
    headingExample: "e.g. Community garden website",
    subheadingExample: "e.g. Designer and developer",
    details: ["Technologies", "Role", "Metrics", "Custom detail"],
  },
  skills: {
    heading: "Skill group name",
    subheading: "Group description",
    headingExample: "e.g. Languages or Tools",
    subheadingExample: "Optional description",
    details: ["Skill"],
  },
  "certifications and licenses": {
    heading: "Certification or license name",
    subheading: "Issuing organization",
    headingExample: "e.g. Certification name",
    subheadingExample: "e.g. Issuing body",
    details: ["Credential ID", "Custom detail"],
  },
  "awards and honors": {
    heading: "Award or honor name",
    subheading: "Issuing organization",
    headingExample: "e.g. Community service award",
    subheadingExample: "e.g. Awarding organization",
    details: ["Recognition", "Metrics", "Custom detail"],
  },
  research: {
    heading: "Research title",
    subheading: "Institution or research group",
    headingExample: "e.g. Research project title",
    subheadingExample: "e.g. Laboratory or institution",
    details: ["Role", "Methods", "Publication", "Custom detail"],
  },
  publications: {
    heading: "Publication title",
    subheading: "Journal or publisher",
    headingExample: "e.g. Article or book title",
    subheadingExample: "e.g. Journal name",
    details: ["Authors", "Volume", "Pages", "Custom detail"],
  },
};

export function entryGuidance(sectionHeading: string): EntryGuidance {
  const key = sectionHeading.trim().toLowerCase();
  return Object.hasOwn(guidance, key) ? guidance[key] : general;
}
