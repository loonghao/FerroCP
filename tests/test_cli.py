"""Test CLI functionality."""

# Import built-in modules
import sys
from unittest import mock
from click.testing import CliRunner

# Import local modules
from py_eacopy import cli


def test_cli_version():
    """Test CLI version command."""
    runner = CliRunner()
    result = runner.invoke(cli.cli, ["--version"])
    assert result.exit_code == 0
    assert "version" in result.output.lower()


def test_cli_copy():
    """Test CLI copy command."""
    runner = CliRunner()
    with runner.isolated_filesystem():
        # Create a test file
        with open("source.txt", "w") as f:
            f.write("test content")

        result = runner.invoke(cli.cli, ["copy", "source.txt", "dest.txt", "--no-progress"])
        assert result.exit_code == 0
        assert "Copy completed successfully" in result.output
        assert "Files copied: 1" in result.output

        # Verify the file was actually copied
        with open("dest.txt", "r") as f:
            assert f.read() == "test content"


def test_cli_copy_with_metadata():
    """Test CLI copy command with preserve metadata."""
    runner = CliRunner()
    with runner.isolated_filesystem():
        # Create a test file
        with open("source.txt", "w") as f:
            f.write("test content")

        result = runner.invoke(cli.cli, ["copy", "source.txt", "dest.txt", "--preserve-metadata", "--no-progress"])
        assert result.exit_code == 0
        assert "Copy completed successfully" in result.output

        # Verify the file was actually copied
        with open("dest.txt", "r") as f:
            assert f.read() == "test content"


def test_cli_copy_directory():
    """Test CLI copy command for directory."""
    runner = CliRunner()
    with runner.isolated_filesystem():
        # Create a test directory with a file
        import os
        os.makedirs("source_dir")
        with open("source_dir/test.txt", "w") as f:
            f.write("test content")

        result = runner.invoke(cli.cli, ["copy", "source_dir", "dest_dir", "--no-progress"])
        assert result.exit_code == 0
        assert "Copy completed successfully" in result.output

        # Verify the directory and file were copied
        assert os.path.exists("dest_dir")
        assert os.path.exists("dest_dir/test.txt")
        with open("dest_dir/test.txt", "r") as f:
            assert f.read() == "test content"


@mock.patch("py_eacopy.EACopy")
def test_cli_copy_with_server(mock_eacopy_class):
    """Test CLI copy-with-server command."""
    # Mock the EACopy instance and its methods
    mock_eacopy = mock_eacopy_class.return_value
    mock_stats = mock.MagicMock()
    mock_stats.bytes_copied = 12
    mock_eacopy.copy_with_server.return_value = mock_stats

    runner = CliRunner()
    with runner.isolated_filesystem():
        # Create a test file
        with open("source.txt", "w") as f:
            f.write("test content")

        result = runner.invoke(cli.cli, ["copy-with-server", "source.txt", "dest.txt", "--server", "server.example.com"])
        assert result.exit_code == 0
        mock_eacopy.copy_with_server.assert_called_once_with("source.txt", "dest.txt", "server.example.com", 8080)
        assert "Network copy completed" in result.output


@mock.patch("py_eacopy.EACopy")
def test_cli_error(mock_eacopy_class):
    """Test CLI error handling."""
    # Mock the EACopy instance to raise an exception
    mock_eacopy = mock_eacopy_class.return_value
    mock_eacopy.copy_file.side_effect = Exception("Test error")

    runner = CliRunner()
    with runner.isolated_filesystem():
        # Create a test file
        with open("source.txt", "w") as f:
            f.write("test content")

        result = runner.invoke(cli.cli, ["copy", "source.txt", "dest.txt"])
        assert result.exit_code == 1
        assert "Error" in result.output
