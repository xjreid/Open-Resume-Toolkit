const paths = {
  refresh:
    "M20 7v5h-5M4 17v-5h5M6.1 7a7 7 0 0 1 11.5-1L20 9M4 15l2.4 3A7 7 0 0 0 17.9 17",
  check: "m8 12 3 3 5-6M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0",
  logout: "M9 4H5v16h4M9 12h12m-4-4 4 4-4 4",
};

export function CodexPageIcon({ name }: { name: keyof typeof paths }) {
  return (
    <svg
      className="plan-icon"
      viewBox="0 0 24 24"
      aria-hidden="true"
      focusable="false"
    >
      <path d={paths[name]} />
    </svg>
  );
}
