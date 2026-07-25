import FitnessPwaApp from "@/src/components/FitnessPwaApp";
import { headers } from "next/headers";
import { redirect } from "next/navigation";

export default async function FitnessPage() {
  const userAgent = (await headers()).get("user-agent") ?? "";
  const isPhone = /iphone|ipod|android.+mobile|windows phone|mobile/i.test(userAgent);
  if (!isPhone) redirect("/");
  return <FitnessPwaApp />;
}
