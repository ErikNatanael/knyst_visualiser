use color_eyre::Result;
use knyst::audio_backend::JackBackend;
use knyst::controller::print_error_handler;
use knyst::envelope::Envelope;
use knyst::gen::delay::static_sample_delay;
use knyst::handles::{handle, Handle};
use knyst::prelude::*;
use knyst::*;
use rand::Rng;
fn main() -> Result<()> {
    // Init Knyst
    // let mut backend = CpalBackend::new(CpalBackendOptions::default())?;
    let mut backend = JackBackend::new("Knyst<3JACK")?;
    let _sphere = KnystSphere::start(
        &mut backend,
        SphereSettings {
            num_inputs: 0,
            num_outputs: 2,
            ..Default::default()
        },
        print_error_handler,
    );

    std::thread::spawn(|| {
        for &freq in [400, 600, 500].iter().cycle() {
            let mut rng = rand::thread_rng();
            // for _ in 0..10 {
            {
                let freq = (sine().freq(
                    sine()
                        .freq(
                            sine()
                                .freq(0.01)
                                .range(0.02, rng.gen_range(0.05..0.3 as Sample)),
                        )
                        .range(0.0, 400.),
                ) * 100.0)
                    + 440.;
                // let freq = sine().freq(0.5).range(200.0, 200.0 * 9.0 / 8.0);
                let node0 = sine();
                node0.freq(freq);
                let modulator = sine();
                modulator.freq(sine().freq(0.09) * -5.0 + 6.0);
                graph_output(0, (node0 * modulator * 0.025).repeat_outputs(1));
            }
            // }
            // new graph
            knyst().init_local_graph(knyst().default_graph_settings());
            let sig = sine().freq(freq as f32).out("sig") * 0.25;
            let env = Envelope {
                points: vec![(1.0, 0.005), (0.0, 0.5)],
                stop_action: StopAction::FreeGraph,
                ..Default::default()
            };
            let sig = sig * handle(env.to_gen());
            // let sig = sig * handle(env.to_gen());

            graph_output(0, sig.repeat_outputs(1));
            // push graph to sphere
            let graph = knyst().upload_local_graph();
            let sig = graph + static_sample_delay(48 * 500).input(graph);

            graph_output(0, sig.repeat_outputs(1));
            std::thread::sleep(std::time::Duration::from_millis(2500));
        }
    });

    // Init visualiser
    knyst_visualiser::init_knyst_visualiser();

    // Create graph

    Ok(())
}
fn sine() -> Handle<OscillatorHandle> {
    oscillator(WavetableId::cos())
}
