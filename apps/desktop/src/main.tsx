import { ProfileBoundary } from "./shared/ProfileBoundary";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./shared/App";
import "./shared/styles/index.css";

const root = document.getElementById("root");
if (!root) throw new Error("Root element is missing");

createRoot(root).render(
  <StrictMode>
    <ProfileBoundary>
      <App surface="main" />
    </ProfileBoundary>
  </StrictMode>,
);
