use crate::generator::Generator;
use crate::types::{LiveCell, Personal};
use ckb_core::transaction::{CellInput, CellOutput, Transaction, TransactionBuilder};
use ckb_core::Bytes;
use ckb_hash::blake2b_256;
use ckb_occupied_capacity::Capacity;
use numext_fixed_hash::H256;
use rand::{thread_rng, Rng};
use std::cmp::max;

pub struct RandomFee;

impl Generator for RandomFee {
    fn generate(
        &self,
        mut live_cells: Vec<LiveCell>,
        sender: &Personal,
        receiver: &Personal,
    ) -> (Vec<LiveCell>, Vec<Transaction>) {
        let rest_cells = if live_cells.len() % 2 == 1 {
            vec![live_cells.pop().unwrap()]
        } else {
            vec![]
        };

        let mut transactions = Vec::new();
        while !live_cells.is_empty() {
            let input_cells: Vec<_> = (0..2).map(|_| live_cells.pop().unwrap()).collect();
            let input_capacities = input_cells.iter().fold(Capacity::zero(), |sum, c| {
                sum.safe_add(c.cell_output.capacity)
                    .expect("sum input capacities")
            });
            let inputs: Vec<_> = input_cells
                .into_iter()
                .map(|c| CellInput::new(c.out_point, 0))
                .collect();
            let outputs = {
                let mut output = CellOutput::new(
                    Capacity::zero(),
                    Bytes::new(),
                    receiver.lock_script().clone(),
                    None,
                );
                output.capacity = output.occupied_capacity().unwrap();
                let mut output2 = output.clone();
                let fee = input_capacities
                    .safe_sub(output.capacity)
                    .expect("input capacity is enough for 2 secp outputs")
                    .safe_sub(output2.capacity)
                    .expect("input capacity is enough for 2 secp outputs");
                let mut rng = thread_rng();
                if fee != Capacity::zero() {
                    output2.capacity = output2
                        .capacity
                        .safe_add(Capacity::shannons(rng.gen_range(0, max(5, fee.as_u64()))))
                        .unwrap();
                }
                vec![output, output2]
            };
            let dep = sender.dep_out_point().clone();
            let raw_transaction = TransactionBuilder::default()
                .inputs(inputs)
                .outputs(outputs)
                .dep(dep)
                .build();
            let witness = {
                let message = H256::from(blake2b_256(raw_transaction.hash()));
                let signature_bytes = sender
                    .privkey()
                    .sign_recoverable(&message)
                    .unwrap()
                    .serialize();
                vec![Bytes::from(signature_bytes)]
            };
            let witnesses = vec![witness.clone(), witness];
            let transaction = TransactionBuilder::from_transaction(raw_transaction)
                .witnesses(witnesses)
                .build();
            transactions.push(transaction);
        }

        (rest_cells, transactions)
    }
}