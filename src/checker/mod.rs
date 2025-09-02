use crate::anchor_info::{AnchorAccountKind, find_to_account_metas, local_anchor_accounts};

pub fn detect_duplicate_mutable_account() {
    let res = find_to_account_metas();
    // println!("{:?}", res);

    let anchor_accounts_collection = local_anchor_accounts();
    // println!("{:?}", anchor_accounts_collection);
    for anchor_accounts in anchor_accounts_collection {
        // println!("{}", anchor_accounts.name);
        let mut muts = vec![];
        for (name, mutability, field_idx) in res.iter() {
            if name == &anchor_accounts.name {
                muts.push((field_idx, mutability));
            }
        }
        let mut final_res = vec![];
        for (idx, anchor_account) in anchor_accounts.anchor_accounts.iter().enumerate() {
            // println!("- {idx}: {:?}", &anchor_account);
            let mut mu = None;
            for (field_idx, mutability) in muts.iter() {
                if *field_idx == &idx {
                    mu = Some(*mutability);
                    break;
                }
            }
            // println!("- {idx}: {:?} {:?}", mu, &anchor_account);
            final_res.push((anchor_account, mu));
        }

        let len = final_res.len();
        for i in 0..len {
            for j in i + 1..len {
                if final_res[i].1 == Some(&"mut") && final_res[j].1 == Some(&"mut") {
                    match (final_res[i].0.kind.clone(), final_res[j].0.kind.clone()) {
                        (
                            AnchorAccountKind::Account(i_struct),
                            AnchorAccountKind::Account(j_struct),
                        ) if i_struct == j_struct => {
                            println!(
                                "Find error: two mutable accounts of the same type in the same Context: {:?} {:?}",
                                final_res[i], final_res[j]
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
