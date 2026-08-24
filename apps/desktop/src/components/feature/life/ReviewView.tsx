import { useState } from "react";
import { Check } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import { dayKey } from "@/src/utils/format";
import { Button } from "@/src/components/ui";

function Range({
  label,
  value,
  set,
}: {
  label: string;
  value: number;
  set: (value: number) => void;
}) {
  return (
    <label className="hx-range">
      <span>
        {label}
        <b>{value}/10</b>
      </span>
      <input
        type="range"
        min="1"
        max="10"
        value={value}
        onChange={(event) => set(Number(event.target.value))}
      />
    </label>
  );
}

export default function ReviewView() {
  const { reviews, saveReview } = useLifeStore();
  const current = reviews.find((item) => item.reviewDate === dayKey());
  const [energy, setEnergy] = useState(current?.energy ?? 7);
  const [mood, setMood] = useState(current?.mood ?? 7);
  const [best, setBest] = useState(current?.bestThing ?? "");
  const [problem, setProblem] = useState(current?.problem ?? "");
  const [priority, setPriority] = useState(current?.tomorrowPriority ?? "");
  const [note, setNote] = useState(current?.note ?? "");
  const [saved, setSaved] = useState(false);

  return (
    <div className="hx-view hx-review">
      <form
        onSubmit={async (event) => {
          event.preventDefault();
          await saveReview({
            energy,
            mood,
            bestThing: best,
            problem,
            tomorrowPriority: priority,
            note,
          });
          setSaved(true);
        }}
      >
        <Range label="今天的精力" value={energy} set={setEnergy} />
        <Range label="今天的心情" value={mood} set={setMood} />
        <label>
          今天做得最好的一件事
          <textarea value={best} onChange={(event) => setBest(event.target.value)} />
        </label>
        <label>
          今天遇到的问题
          <textarea
            value={problem}
            onChange={(event) => setProblem(event.target.value)}
          />
        </label>
        <label>
          明天最重要的一件事
          <input
            value={priority}
            onChange={(event) => setPriority(event.target.value)}
          />
        </label>
        <label>
          补充备注
          <textarea value={note} onChange={(event) => setNote(event.target.value)} />
        </label>
        <Button variant="primary" type="submit">
          {saved ? (
            <>
              <Check aria-hidden="true" /> 已保存今天
            </>
          ) : (
            "保存今日复盘"
          )}
        </Button>
      </form>
    </div>
  );
}
