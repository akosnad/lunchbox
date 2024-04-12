import { useEffect, useState } from "preact/hooks";

const maxValue = 255;

function DmxChannel({ channel }) {
  const ratio = Math.round((channel.value / maxValue) * 100).toString();
  return (
    <div
      className="channel"
      style={`background: linear-gradient(to top, blue ${ratio}%, transparent ${ratio}%`}
    >
      <p>{channel.channel}</p>
    </div>
  );
}

export function Dmx() {
  const [state, setState] = useState(null);
  const [active, setActive] = useState(false);

  const fetchDmx = async () => {
    const response = await fetch("/dmx", {
      method: "GET",
      cache: "no-cache",
      responseType: "arraybuffer",
      headers: {
        "Content-Type": "application/octet-stream",
      },
    });
    console.log(response);
    if (!response.ok) {
      console.error("Error fetching DMX data: ", response.statusText);
      return;
    }
    // put the ReadableStream into an uint8 array
    const raw = new Uint8Array(await response.arrayBuffer());
    // parse the raw data into a list of channels
    const channels = [];
    for (let i = 0; i < raw.length; i++) {
      channels.push({ channel: i + 1, value: raw[i] });
    }
    setState(channels);
  };

  useEffect(() => {
    if (!active) return;
    const i = setInterval(fetchDmx, 100);
    return () => clearInterval(i);
  }, [active]);

  return (
    <>
      <button onClick={() => setActive((active) => !active)}>
        {active ? "Stop reading DMX data" : "Start reading DMX data"}
      </button>

      <div className="channels">
        {state ? (
          <>
            {state.map((channel, index) => (
              <DmxChannel key={index} channel={channel} />
            ))}
          </>
        ) : (
          <p>No data</p>
        )}
      </div>
    </>
  );
}
