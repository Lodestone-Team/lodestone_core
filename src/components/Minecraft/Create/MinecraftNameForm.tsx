import InputField from 'components/Atoms/Form/InputField';
import Textfield from 'components/Atoms/Config/InputBox';

export default function MinecraftNameForm() {
  return (
    <>
      <h1 className="text-larger font-bold tracking-tight text-gray-300">
        Create an Instance
      </h1>
      <p>Create a new Minecraft server instance to play with your friends.</p>
      <div className="mt-10 flex flex-col gap-16 text-left">
        <InputField type="text" name="name" label="Name" />
      </div>
    </>
  );
}