import Image from "next/image";
import { SparkleBackground } from "@/components/background/sparkle-background";
import { Hack26Header } from "@/components/layout/hack26-header";
import { IntentDeskPanel } from "@/components/desk/intent-desk-panel";

export default function Home() {
  return (
    <main className="relative min-h-[200vh] w-full">
      <Image
        src="/chainlink-background.png"
        alt=""
        fill
        priority
        sizes="100vw"
        className="fixed inset-0 object-cover"
      />
      <SparkleBackground />
      <Hack26Header />
      <IntentDeskPanel />
      <section id="publish-receipt" className="intent-desk-panel app-anchor-panel">
        <div className="intent-desk-head">
          <p>Publish Receipt</p>
          <span>Onchain publish status and tx receipt</span>
        </div>
        <div className="intent-result">
          <p>Waiting for proof pipeline to reach PUBLISHING/PUBLISHED.</p>
        </div>
      </section>
      <section id="audit" className="intent-desk-panel app-anchor-panel">
        <div className="intent-desk-head">
          <p>Audit Trail</p>
          <span>Deterministic event trail for workflow evidence</span>
        </div>
        <div className="intent-result">
          <p>Audit view placeholder is active.</p>
        </div>
      </section>
      <section id="health" className="intent-desk-panel app-anchor-panel">
        <div className="intent-desk-head">
          <p>System Health</p>
          <span>Coordinator queue + worker + dependency health</span>
        </div>
        <div className="intent-result">
          <p>Health panel placeholder is active.</p>
        </div>
      </section>
      <div className="relative z-10 h-[120vh]" aria-hidden />
    </main>
  );
}
