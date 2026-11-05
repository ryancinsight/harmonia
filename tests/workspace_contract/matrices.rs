use super::*;

#[test]
fn replacement_model_is_rechecked_and_the_pair_recovers_after_rejection() {
    let calls = Rc::new(Cell::new(0));
    let model = PairComponents::new(
        sized_partition(0.5, &calls),
        sized_partition(0.25, &calls),
        Directional::Identity,
        Directional::Identity,
        FullRelaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("compatible initial model");
    let mut pair =
        PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("positive subcycles");
    let mut replacement = sized_partition(0.5, &calls);
    replacement.output = 2;
    *pair.model_mut() = PairComponents::new(
        replacement,
        sized_partition(0.25, &calls),
        Directional::Select,
        Directional::Identity,
        FullRelaxation,
    );
    let (mut first_state, mut second_state, mut first_input, mut second_input) =
        ([1.0], [2.0], [0.0], [0.0]);
    let before = (first_state, second_state, first_input, second_input);
    let policy = ConvergencePolicy::new(1e-12, 1e-12, 100).expect("valid policy");
    let result = pair.solve_window(
        instant(),
        window(),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut NoObserver,
    );
    assert!(matches!(
        result,
        Err(CouplingError::Dimension {
            role: "first workspace output",
            expected: 2,
            actual: 1
        })
    ));
    assert_eq!(calls.get(), 0);
    assert_eq!(
        (first_state, second_state, first_input, second_input),
        before
    );

    *pair.model_mut() = PairComponents::new(
        sized_partition(0.5, &calls),
        sized_partition(0.25, &calls),
        Directional::Identity,
        Directional::Identity,
        FullRelaxation,
    );
    let report = pair
        .solve_window(
            instant(),
            window(),
            &mut first_state,
            &mut second_state,
            &mut first_input,
            &mut second_input,
            &policy,
            &mut NoObserver,
        )
        .expect("the restored compatible model converges");
    assert!(report.iterations > 1);
    for value in [
        first_state[0],
        second_state[0],
        first_input[0],
        second_input[0],
    ] {
        assert!((value - 4.0).abs() < 1e-9);
    }
}

#[test]
fn transfer_destination_is_rechecked_after_partition_callbacks() {
    for (first_direction, role) in [
        (true, "first-to-second transfer destination"),
        (false, "second-to-first transfer destination"),
    ] {
        let calls = Rc::new(Cell::new(0));
        let dimension = Rc::new(Cell::new(1));
        let transfer_calls = Rc::new(Cell::new(0));
        let mut first = sized_partition(0.5, &calls);
        first.change_on_advance = Some(Rc::clone(&dimension));
        let second = sized_partition(0.25, &calls);
        let adaptive = Directional::Adaptive {
            dimension: Rc::clone(&dimension),
            calls: Rc::clone(&transfer_calls),
        };
        let (forward, reverse) = if first_direction {
            (adaptive, Directional::Identity)
        } else {
            (Directional::Identity, adaptive)
        };
        let model = PairComponents::new(first, second, forward, reverse, FullRelaxation);
        let workspace =
            PairWorkspace::for_model(&model).expect("initial transfer dimensions match");
        let mut pair =
            PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("positive subcycles");
        let (mut first_state, mut second_state, mut first_input, mut second_input) =
            ([1.0], [2.0], [3.0], [4.0]);
        let before = (first_state, second_state, first_input, second_input);
        let policy = ConvergencePolicy::new(1e-12, 1e-12, 100).expect("valid policy");
        let result = catch_unwind(AssertUnwindSafe(|| {
            pair.solve_window(
                instant(),
                window(),
                &mut first_state,
                &mut second_state,
                &mut first_input,
                &mut second_input,
                &policy,
                &mut NoObserver,
            )
        }))
        .expect("a changed destination must not reach an undersized scratch slice");
        assert!(
            matches!(result, Err(CouplingError::Dimension { role: actual_role, expected: 1, actual: 2 }) if actual_role == role)
        );
        assert_eq!(transfer_calls.get(), 0);
        assert!(calls.get() > 0);
        assert_eq!(
            (first_state, second_state, first_input, second_input),
            before
        );
    }
}

#[test]
fn every_foreign_workspace_dimension_is_rejected_before_partition_callbacks() {
    for (slot, role) in [
        "first workspace state",
        "second workspace state",
        "first workspace input",
        "second workspace input",
        "first workspace output",
        "second workspace output",
    ]
    .into_iter()
    .enumerate()
    {
        let calls = Rc::new(Cell::new(0));
        let mut first = sized_partition(0.5, &calls);
        let mut second = sized_partition(0.25, &calls);
        let (mut first_transfer, mut second_transfer) =
            (Directional::Identity, Directional::Identity);
        match slot {
            0 => first.state = 2,
            1 => second.state = 2,
            2 => {
                first.input = 2;
                second_transfer = Directional::Replicate;
            }
            3 => {
                second.input = 2;
                first_transfer = Directional::Replicate;
            }
            4 => {
                first.output = 2;
                first_transfer = Directional::Select;
            }
            5 => {
                second.output = 2;
                second_transfer = Directional::Select;
            }
            _ => unreachable!("six dimensions"),
        }
        let sizes = [first.state, second.state, first.input, second.input];
        let model = PairComponents::new(
            first,
            second,
            first_transfer,
            second_transfer,
            FullRelaxation,
        );
        let mut pair = PartitionedPair::<_, f64, 1, 1>::new(model, unit_workspace())
            .expect("positive subcycles");
        let mut first_state = vec![1.0; sizes[0]];
        let mut second_state = vec![2.0; sizes[1]];
        let mut first_input = vec![3.0; sizes[2]];
        let mut second_input = vec![4.0; sizes[3]];
        let before = (
            first_state.clone(),
            second_state.clone(),
            first_input.clone(),
            second_input.clone(),
        );
        let policy = ConvergencePolicy::new(1e-12, 1e-12, 100).expect("valid policy");
        let result = pair.solve_window(
            instant(),
            window(),
            &mut first_state,
            &mut second_state,
            &mut first_input,
            &mut second_input,
            &policy,
            &mut NoObserver,
        );
        assert!(
            matches!(result, Err(CouplingError::Dimension { role: actual_role, expected: 2, actual: 1 }) if actual_role == role)
        );
        assert_eq!(
            calls.get(),
            0,
            "no checkpoint, advance, or export for {role}"
        );
        assert_eq!(
            (first_state, second_state, first_input, second_input),
            before
        );
    }
}

#[test]
fn incompatible_transfer_declarations_are_rejected_before_partition_callbacks() {
    for (first_direction, role) in [
        (true, "first-to-second transfer destination"),
        (false, "second-to-first transfer destination"),
    ] {
        let calls = Rc::new(Cell::new(0));
        let first = sized_partition(0.5, &calls);
        let second = sized_partition(0.25, &calls);
        let (forward, reverse) = if first_direction {
            (Directional::Replicate, Directional::Identity)
        } else {
            (Directional::Identity, Directional::Replicate)
        };
        let model = PairComponents::new(first, second, forward, reverse, FullRelaxation);
        let mut pair = PartitionedPair::<_, f64, 1, 1>::new(model, unit_workspace())
            .expect("positive subcycles");
        let (mut first_state, mut second_state, mut first_input, mut second_input) =
            ([1.0], [2.0], [3.0], [4.0]);
        let before = (first_state, second_state, first_input, second_input);
        let policy = ConvergencePolicy::new(1e-12, 1e-12, 100).expect("valid policy");
        let result = pair.solve_window(
            instant(),
            window(),
            &mut first_state,
            &mut second_state,
            &mut first_input,
            &mut second_input,
            &policy,
            &mut NoObserver,
        );
        assert!(
            matches!(result, Err(CouplingError::Dimension { role: actual_role, expected: 1, actual: 2 }) if actual_role == role)
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(
            (first_state, second_state, first_input, second_input),
            before
        );
    }
}

#[test]
fn short_transfer_results_are_typed_errors_and_keep_caller_values() {
    for (first_direction, role) in [
        (true, "first-to-second transfer result"),
        (false, "second-to-first transfer result"),
    ] {
        let calls = Rc::new(Cell::new(0));
        let mut first = sized_partition(0.5, &calls);
        let mut second = sized_partition(0.25, &calls);
        let (forward, reverse) = if first_direction {
            first.output = 2;
            second.input = 2;
            (Directional::Truncated, Directional::Identity)
        } else {
            second.output = 2;
            first.input = 2;
            (Directional::Identity, Directional::Truncated)
        };
        let sizes = [first.state, second.state, first.input, second.input];
        let model = PairComponents::new(first, second, forward, reverse, FullRelaxation);
        let workspace =
            PairWorkspace::for_model(&model).expect("declared transfer dimensions match");
        let mut pair =
            PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("positive subcycles");
        let mut first_state = vec![1.0; sizes[0]];
        let mut second_state = vec![2.0; sizes[1]];
        let mut first_input = vec![3.0; sizes[2]];
        let mut second_input = vec![4.0; sizes[3]];
        let before = (
            first_state.clone(),
            second_state.clone(),
            first_input.clone(),
            second_input.clone(),
        );
        let policy = ConvergencePolicy::new(1e-12, 1e-12, 100).expect("valid policy");
        let result = catch_unwind(AssertUnwindSafe(|| {
            pair.solve_window(
                instant(),
                window(),
                &mut first_state,
                &mut second_state,
                &mut first_input,
                &mut second_input,
                &policy,
                &mut NoObserver,
            )
        }))
        .expect("a short transfer result must not panic");
        assert!(
            matches!(result, Err(CouplingError::Dimension { role: actual_role, expected: 2, actual: 1 }) if actual_role == role)
        );
        assert!(
            calls.get() > 0,
            "real partitions were evaluated before the bad transfer"
        );
        assert_eq!(
            (first_state, second_state, first_input, second_input),
            before
        );
    }
}

#[test]
fn compatible_replication_transfer_solves_the_linear_system() {
    let calls = Rc::new(Cell::new(0));
    let mut first = sized_partition(0.5, &calls);
    first.input = 2;
    let second = sized_partition(0.25, &calls);
    let model = PairComponents::new(
        first,
        second,
        Directional::Identity,
        Directional::Replicate,
        FullRelaxation,
    );
    let workspace =
        PairWorkspace::for_model(&model).expect("replication satisfies the receiving dimension");
    let mut pair =
        PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("positive subcycles");
    let (mut first_state, mut second_state, mut first_input, mut second_input) =
        ([1.0], [2.0], [0.0, 0.0], [0.0]);
    let policy = ConvergencePolicy::new(1e-12, 1e-12, 100).expect("valid policy");
    let report = pair
        .solve_window(
            instant(),
            window(),
            &mut first_state,
            &mut second_state,
            &mut first_input,
            &mut second_input,
            &policy,
            &mut NoObserver,
        )
        .expect("the contracting system converges");
    assert!(report.iterations > 1);
    assert!((first_state[0] - 20.0 / 3.0).abs() < 1e-9);
    assert!((second_state[0] - 14.0 / 3.0).abs() < 1e-9);
    assert!(first_input.iter().all(|v| (*v - 14.0 / 3.0).abs() < 1e-9));
    assert!((second_input[0] - 20.0 / 3.0).abs() < 1e-9);
    assert!(calls.get() > 0);
}
