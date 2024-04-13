import { useEffect, useRef, useState } from "preact/hooks";

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
  const socket = useRef(null);

  const parseDmx = async (event) => {
    const data = event.data;
    const buf = await data.arrayBuffer();
    const arr = new Uint8Array(buf);
    setState(
      Array.from(arr).map((value, index) => {
        return {
          channel: index + 1,
          value: value,
        };
      })
    );
  };

  const fetchDmx = async () => {
    const domain = window.location.hostname;
    const port = window.location.port;
    socket.current = new WebSocket(`ws://${domain}:${port}/ws/dmx`);
    socket.current.onmessage = parseDmx;
  };

  useEffect(() => {
    if (active) {
      fetchDmx();
    } else {
      if (socket.current) socket.current.close();
    }
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
