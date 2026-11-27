import { useEffect, useState } from "react";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { checkWslMaxima, listWslDistros, maximaStatus, type MaximaLocation } from "../../lib/config-client";
import { useLogStore } from "../../store/logStore";
import { ChoiceButtons, SettingRow, TextSetting, type Label, type SectionProps } from "./SettingRow";

const EVAL_TIMEOUTS = [10, 30, 60, 120];
const BACKENDS: { value: string; label: Label }[] = [
  {
    value: "local",
    label: msg({ message: "Local", comment: "Settings: backend button; runs Maxima installed on this computer" }),
  },
  { value: "docker", label: "Docker" },
  { value: "wsl", label: "WSL" },
];
const CONTAINER_ENGINES: { value: string; label: Label }[] = [
  { value: "docker", label: "Docker" },
  { value: "podman", label: "Podman" },
];

export function MaximaSection({ config, update }: SectionProps) {
  const { t } = useLingui();
  const exampleImage = "aximar/maxima";

  return (
    <>
      <SettingRow
        label={t({ message: "Backend", comment: "Settings: the label of the buttons that choose where Maxima runs" })}
        hint={t({
          message: "Where Maxima runs: on this computer, in a container, or under WSL.",
          comment: "Settings, Maxima section: hint under Backend",
        })}
      >
        {(ids) => <ChoiceButtons {...ids} options={BACKENDS} value={config.backend} onChange={(backend) => update({ backend })} />}
      </SettingRow>

      {config.backend === "local" && (
        <SettingRow
          label={t({ message: "Maxima path", comment: "Settings: the label of the field for the location of the maxima program" })}
          hint={t({
            message: "Leave it empty and Aximar finds Maxima itself.",
            comment: "Settings, Maxima section: hint under Maxima path",
          })}
        >
          {(ids) => (
            <TextSetting
              {...ids}
              value={config.maxima_path ?? ""}
              placeholder={t({
                message: "Auto-detect",
                comment: "Settings: placeholder in the Maxima path field; when empty, Aximar finds maxima itself",
              })}
              onSave={(path) => update({ maxima_path: path.trim() || null })}
            />
          )}
        </SettingRow>
      )}
      {config.backend === "local" && <MaximaStatusLine maximaPath={config.maxima_path} />}

      {config.backend === "docker" && (
        <>
          <SettingRow
            label={t({
              message: "Container engine",
              comment: "Settings: the label of the buttons that choose the program that runs the Maxima container",
            })}
            hint={t({
              message: "The program that runs the Maxima container.",
              comment: "Settings, Maxima section: hint under Container engine",
            })}
          >
            {(ids) => (
              <ChoiceButtons
                {...ids}
                options={CONTAINER_ENGINES}
                value={config.container_engine}
                onChange={(container_engine) => update({ container_engine })}
              />
            )}
          </SettingRow>
          <SettingRow
            label={t({ message: "Docker image", comment: "Settings: the label of the field for the container image that holds Maxima" })}
            hint={t({ message: "The container image that holds Maxima.", comment: "Settings, Maxima section: hint under Docker image" })}
          >
            {(ids) => (
              <TextSetting
                {...ids}
                value={config.docker_image}
                placeholder={t({
                  message: `e.g. ${exampleImage}`,
                  comment: "Settings: placeholder in the Docker image field. exampleImage is an image name, such as aximar/maxima",
                })}
                onSave={(docker_image) => update({ docker_image })}
              />
            )}
          </SettingRow>
        </>
      )}

      {config.backend === "wsl" && <WslDistroRow config={config} update={update} />}

      <SettingRow
        label={t({
          message: "Eval timeout",
          comment: "Settings: the label of the menu that chooses how long a cell may run before Aximar stops it",
        })}
        hint={t({
          message: "How long a cell may run before Aximar stops it.",
          comment: "Settings, Maxima section: hint under Eval timeout",
        })}
      >
        {({ id, describedBy }) => (
          <select
            id={id}
            aria-describedby={describedBy}
            className="settings-select"
            value={config.eval_timeout}
            onChange={(e) => update({ eval_timeout: Number(e.target.value) })}
          >
            {EVAL_TIMEOUTS.map((secs) => (
              <option key={secs} value={secs}>
                {secs}s
              </option>
            ))}
          </select>
        )}
      </SettingRow>
    </>
  );
}

function WslDistroRow({ config, update }: SectionProps) {
  const { t } = useLingui();
  const addLogEntry = useLogStore((s) => s.addEntry);
  const [distros, setDistros] = useState<string[]>([]);
  /** Where maxima is in the distro; null if it has none; undefined while checking */
  const [found, setFound] = useState<string | null | undefined>(undefined);

  useEffect(() => {
    listWslDistros()
      .then(setDistros)
      .catch((e) => {
        addLogEntry("error", `Failed to list WSL distros: ${e}`, "settings");
        setDistros([]);
      });
  }, [addLogEntry]);

  useEffect(() => {
    setFound(undefined);
    checkWslMaxima(config.wsl_distro)
      .then(setFound)
      .catch((e) => {
        addLogEntry("error", `Failed to check WSL Maxima: ${e}`, "settings");
        setFound(null);
      });
  }, [config.wsl_distro, addLogEntry]);

  return (
    <SettingRow
      label={t({
        message: "WSL distro",
        comment: "Settings: the label of the menu that chooses the Linux distribution under WSL that runs Maxima",
      })}
      hint={t({ message: "The Linux distribution that runs Maxima.", comment: "Settings, Maxima section: hint under WSL distro" })}
    >
      {({ id, describedBy }) => (
        <>
          <select
            id={id}
            aria-describedby={describedBy}
            className="settings-select"
            value={config.wsl_distro}
            onChange={(e) => update({ wsl_distro: e.target.value })}
          >
            <option value="">
              {t({ message: "Default", comment: "Settings: WSL distro menu item for the distribution WSL uses by default" })}
            </option>
            {distros.map((d) => (
              <option key={d} value={d}>
                {d}
              </option>
            ))}
          </select>
          <span
            className={`settings-wsl-status${found === undefined ? " checking" : found ? " found" : " missing"}`}
            title={
              found === undefined
                ? t({ message: "Checking...", comment: "Settings: tooltip beside the WSL distro menu while Aximar looks for maxima in it" })
                : found
                  ? t({
                      message: `Found: ${found}`,
                      comment: "Settings: tooltip beside the WSL distro menu. found is where the maxima program is in the distro",
                    })
                  : t({
                      message: "maxima not found in this distro",
                      comment: "Settings: tooltip beside the WSL distro menu when the distro has no maxima program",
                    })
            }
          >
            {found === undefined
              ? ""
              : found
                ? t({ message: "maxima found", comment: "Settings: shown beside the WSL distro menu when the distro has the maxima program" })
                : t({ message: "maxima not found", comment: "Settings: shown beside the WSL distro menu when the distro has no maxima program" })}
          </span>
        </>
      )}
    </SettingRow>
  );
}

/**
 * Which Maxima the Local backend would run, and where its path came from.
 * Asks again when the Maxima path changes.
 */
function MaximaStatusLine({ maximaPath }: { maximaPath: string | null }) {
  const { t } = useLingui();
  const [location, setLocation] = useState<MaximaLocation | null>(null);

  useEffect(() => {
    let current = true;
    maximaStatus(maximaPath)
      .then((l) => current && setLocation(l))
      .catch(() => current && setLocation(null));
    return () => {
      current = false;
    };
  }, [maximaPath]);

  if (!location) return null;
  const path = location.path ?? "";
  const envVar = "AXIMAR_MAXIMA_PATH";
  let text: string;
  let tone: "ok" | "err";
  if (location.problem === "not-found" && location.path) {
    tone = "err";
    text = t({
      message: `${path} doesn't exist.`,
      comment: "Settings, Maxima section: status line when the Maxima path names no file. path is the path",
    });
  } else if (location.problem === "not-executable") {
    tone = "err";
    text = t({
      message: `${path} isn't a program Aximar can run.`,
      comment: "Settings, Maxima section: status line when the Maxima path names a file that can't be run. path is the path",
    });
  } else if (!location.path) {
    tone = "err";
    text = t({
      message: "Aximar found no Maxima. Install it, or set the Maxima path above.",
      comment: "Settings, Maxima section: status line when Aximar can't find Maxima anywhere",
    });
  } else {
    tone = "ok";
    switch (location.source) {
      case "setting":
        text = t({
          message: `Aximar runs ${path}, from the Maxima path setting.`,
          comment: "Settings, Maxima section: status line. path is the Maxima program Aximar starts",
        });
        break;
      case "environment":
        text = t({
          message: `Aximar runs ${path}, from the ${envVar} environment variable.`,
          comment: "Settings, Maxima section: status line. path is the Maxima program; envVar is an environment variable's name",
        });
        break;
      case "standard-location":
        text = t({
          message: `Aximar runs ${path}, found in a standard install location.`,
          comment: "Settings, Maxima section: status line. path is the Maxima program Aximar found",
        });
        break;
      default:
        text = t({
          message: `Aximar runs ${path}, found on PATH.`,
          comment: "Settings, Maxima section: status line. path is the Maxima program; PATH is the list of folders the system searches for programs",
        });
    }
  }
  return (
    <p className={`settings-status-line ${tone}`} role="status" data-status="maxima">
      {text}
    </p>
  );
}
