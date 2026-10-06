import { ProfileBoundary } from "./shared/ProfileBoundary";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./shared/App";
import { ApplicationPopup } from "./shared/ApplicationPopup";
import "./shared/styles/index.css";

const root = document.getElementById("root");
if (!root) throw new Error("Root element is missing");

createRoot(root).render(
  <StrictMode>
    <ProfileBoundary>
      {new URLSearchParams(window.location.search).get("popup") === "1" ? (
        <ApplicationPopup />
      ) : (
        <App surface="overlay" />
      )}
    </ProfileBoundary>
  </StrictMode>,
);
