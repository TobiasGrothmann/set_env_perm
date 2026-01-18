use set_env_perm;

fn main() {
    // set_env_perm::check_or_set("d", "world6777").expect("not working");
    set_env_perm::set("XXX", "man").unwrap();
    set_env_perm::set("XXXd", "man").unwrap();
    set_env_perm::append("PATH", "hello").unwrap();
    set_env_perm::prepend("PATH", "okkkk").unwrap();
}
