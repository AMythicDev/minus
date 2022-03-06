use minus::error::MinusError;
use std::time::Duration;
use tokio::{join, task::spawn_blocking, time::sleep};

#[tokio::main]
async fn main() -> Result<(), MinusError> {
    let output = minus::Pager::new();

    let increment = async {
        for i in 0..=10_u32 {
            output.push_str(&format!("{}\n", i))?;
            sleep(Duration::from_millis(100)).await;
        }
        output.send_message("No more output to come")?;
        Result::<_, MinusError>::Ok(())
    };

    let output = output.clone();
    let (res1, res2) = join!(
        spawn_blocking(move || minus::dynamic_paging(output)),
        increment
    );
    res1.unwrap()?;
    res2?;
    Ok(())
}
