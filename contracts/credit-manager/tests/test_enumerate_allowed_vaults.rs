use crate::helpers::MockEnv;

pub mod helpers;

#[test]
fn test_pagination_on_allowed_vaults_query_works() {
    let allowed_vaults = vec![
        "addr1".to_string(),
        "addr2".to_string(),
        "addr3".to_string(),
        "addr4".to_string(),
        "addr5".to_string(),
        "addr6".to_string(),
        "addr7".to_string(),
        "addr8".to_string(),
        "addr9".to_string(),
        "addr10".to_string(),
        "addr11".to_string(),
        "addr12".to_string(),
        "addr13".to_string(),
        "addr14".to_string(),
        "addr15".to_string(),
        "addr16".to_string(),
        "addr17".to_string(),
        "addr18".to_string(),
        "addr19".to_string(),
        "addr20".to_string(),
        "addr21".to_string(),
        "addr22".to_string(),
        "addr23".to_string(),
        "addr24".to_string(),
        "addr25".to_string(),
        "addr26".to_string(),
        "addr27".to_string(),
        "addr28".to_string(),
        "addr29".to_string(),
        "addr30".to_string(),
        "addr31".to_string(),
        "addr32".to_string(),
    ];

    let mock = MockEnv::new()
        .allowed_vaults(&allowed_vaults)
        .build()
        .unwrap();

    let vaults_res = mock.query_allowed_vaults(None, Some(58_u32));

    // Assert maximum is observed
    assert_eq!(vaults_res.len(), 30);

    let vaults_res = mock.query_allowed_vaults(None, Some(2_u32));

    // Assert limit request is observed
    assert_eq!(vaults_res.len(), 2);

    let vaults_res_a = mock.query_allowed_vaults(None, None);
    let vaults_res_b = mock.query_allowed_vaults(Some(vaults_res_a.last().unwrap().clone()), None);
    let vaults_res_c = mock.query_allowed_vaults(Some(vaults_res_b.last().unwrap().clone()), None);
    let vaults_res_d = mock.query_allowed_vaults(Some(vaults_res_c.last().unwrap().clone()), None);

    // Assert default is observed
    assert_eq!(vaults_res_a.len(), 10);
    assert_eq!(vaults_res_b.len(), 10);
    assert_eq!(vaults_res_c.len(), 10);

    assert_eq!(vaults_res_d.len(), 2);

    let combined: Vec<String> = vaults_res_a
        .iter()
        .cloned()
        .chain(vaults_res_b.iter().cloned())
        .chain(vaults_res_c.iter().cloned())
        .chain(vaults_res_d.iter().cloned())
        .collect();

    assert_eq!(combined.len(), allowed_vaults.len());
    assert!(allowed_vaults.iter().all(|item| combined.contains(item)));
}
